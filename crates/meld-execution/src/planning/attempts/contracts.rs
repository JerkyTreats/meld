//! Public contracts for durable execution planning attempt audit records.

use crate::planning::lowering::Diagnostic as LoweringDiagnostic;
use crate::planning::{
    MethodCandidateReport, PlanningDiagnostic, PlanningRequestIdentity, PlanningWorldStateRequest,
};
use crate::task_network::command;
use serde::{Deserialize, Serialize};

/// Current durable planning attempt record schema.
pub const PLANNING_ATTEMPT_SCHEMA_VERSION: u32 = 1;

/// Current schema for one non-authoritative planning decision audit payload.
pub const PLANNING_ATTEMPT_DECISION_SCHEMA_VERSION: u32 = 1;

/// Hard upper bound for one planning attempt history or selector query.
pub const MAX_PLANNING_ATTEMPT_QUERY_ITEMS: usize = 256;

const MAX_DIAGNOSTIC_CODE_BYTES: usize = 128;
const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 1024;
const MAX_AUDIT_ITEMS: usize = 256;
const MAX_AUDIT_BYTES: usize = 256 * 1024;
const PLANNING_ATTEMPT_ID_DOMAIN: &[u8] = b"meld.execution.planning-attempt.v1";
const PLANNING_COMMAND_ID_DOMAIN: &[u8] = b"meld.execution.planning-command.v1";
const PLANNING_RESPONSE_HASH_DOMAIN: &[u8] = b"meld.execution.planning-command-response.v1";
const PLANNING_DECISION_HASH_DOMAIN: &[u8] = b"meld.execution.planning-decision-audit.v1";
const PLANNING_RECORD_HASH_DOMAIN: &[u8] = b"meld.execution.planning-attempt-record.v1";
const PLANNING_CONTINUATION_HASH_DOMAIN: &[u8] = b"meld.execution.planning-continuation.v1";

/// Durable owner capability that fences one planning actor generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningAttemptOwnerFence {
    runtime_id: String,
    lease_id: String,
    epoch: u64,
}

impl PlanningAttemptOwnerFence {
    pub(crate) fn issued(runtime_id: String, lease_id: String, epoch: u64) -> Result<Self, String> {
        let fence = Self {
            runtime_id,
            lease_id,
            epoch,
        };
        fence.validate()?;
        Ok(fence)
    }

    /// Validate the exact runtime lease identity and monotonic owner epoch.
    pub fn validate(&self) -> Result<(), String> {
        validate_bounded_text("planning owner runtime id", &self.runtime_id, 256)?;
        validate_bounded_text("planning owner lease id", &self.lease_id, 256)?;
        if self.epoch == 0 {
            return Err("planning owner epoch must be positive".to_string());
        }
        Ok(())
    }

    /// Borrow the runtime id whose actor generation owns planning writes.
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    /// Borrow the supervisor lease id bound to this planning owner.
    pub fn lease_id(&self) -> &str {
        &self.lease_id
    }

    /// Return the monotonic planning owner epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// Exact inputs for an attempt that terminates because projection failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningProjectionFailureIdentityInputsWire")]
pub struct PlanningProjectionFailureIdentityInputs {
    goal_id: String,
    goal_updated_at_seq: u64,
    projection_request_hash: String,
    method_library_digest: String,
    capability_catalog_digest: String,
    planning_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PlanningProjectionFailureIdentityInputsWire {
    goal_id: String,
    goal_updated_at_seq: u64,
    projection_request_hash: String,
    method_library_digest: String,
    capability_catalog_digest: String,
    planning_version: String,
}

impl PlanningProjectionFailureIdentityInputs {
    /// Bind a failed projection attempt to its exact request and planning environment.
    pub fn bind(
        projection_request: PlanningWorldStateRequest,
        method_library_digest: String,
        capability_catalog_digest: String,
        planning_version: String,
    ) -> Result<Self, String> {
        let projection_request_hash = projection_request.canonical_hash()?;
        let inputs = Self {
            goal_id: projection_request.goal_id,
            goal_updated_at_seq: projection_request.source_seq,
            projection_request_hash,
            method_library_digest,
            capability_catalog_digest,
            planning_version,
        };
        inputs.validate()?;
        Ok(inputs)
    }

    /// Validate the exact request hash and planning environment identity.
    pub fn validate(&self) -> Result<(), String> {
        validate_bounded_text("projection failure goal id", &self.goal_id, 256)?;
        if self.goal_updated_at_seq == 0 {
            return Err("projection failure goal update sequence must be positive".to_string());
        }
        validate_digest("projection failure request", &self.projection_request_hash)?;
        validate_digest("method library", &self.method_library_digest)?;
        validate_digest("capability catalog", &self.capability_catalog_digest)?;
        validate_bounded_text("planning version", &self.planning_version, 256)
    }

    /// Borrow the goal id whose exact projection request failed.
    pub fn goal_id(&self) -> &str {
        &self.goal_id
    }

    /// Return the goal update sequence carried by the failed projection request.
    pub fn goal_updated_at_seq(&self) -> u64 {
        self.goal_updated_at_seq
    }

    /// Borrow the canonical hash of the failed projection request.
    pub fn projection_request_hash(&self) -> &str {
        &self.projection_request_hash
    }
}

impl TryFrom<PlanningProjectionFailureIdentityInputsWire>
    for PlanningProjectionFailureIdentityInputs
{
    type Error = String;

    fn try_from(value: PlanningProjectionFailureIdentityInputsWire) -> Result<Self, Self::Error> {
        let inputs = Self {
            goal_id: value.goal_id,
            goal_updated_at_seq: value.goal_updated_at_seq,
            projection_request_hash: value.projection_request_hash,
            method_library_digest: value.method_library_digest,
            capability_catalog_digest: value.capability_catalog_digest,
            planning_version: value.planning_version,
        };
        inputs.validate()?;
        Ok(inputs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum PlanningAttemptSourceIdentity {
    ProjectionCompleted {
        planning_request: Box<PlanningRequestIdentity>,
    },
    ProjectionFailed {
        inputs: PlanningProjectionFailureIdentityInputs,
    },
}

/// Complete immutable identity for one execution planning attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningAttemptIdentityWire")]
pub struct PlanningAttemptIdentity {
    attempt_id: String,
    source: PlanningAttemptSourceIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PlanningAttemptIdentityWire {
    attempt_id: String,
    source: PlanningAttemptSourceIdentity,
}

impl PlanningAttemptIdentity {
    /// Bind an attempt to the exact deterministic planning request identity.
    pub fn bind(planning_request: PlanningRequestIdentity) -> Result<Self, String> {
        planning_request.validate()?;
        let mut identity = Self {
            attempt_id: String::new(),
            source: PlanningAttemptSourceIdentity::ProjectionCompleted {
                planning_request: Box::new(planning_request),
            },
        };
        identity.attempt_id = derive_attempt_id(&identity)?;
        identity.validate()?;
        Ok(identity)
    }

    /// Bind an attempt that failed before a projection frame could exist.
    pub fn bind_projection_failure(
        inputs: PlanningProjectionFailureIdentityInputs,
    ) -> Result<Self, String> {
        inputs.validate()?;
        let mut identity = Self {
            attempt_id: String::new(),
            source: PlanningAttemptSourceIdentity::ProjectionFailed { inputs },
        };
        identity.attempt_id = derive_attempt_id(&identity)?;
        identity.validate()?;
        Ok(identity)
    }

    /// Validate the derived id and its one canonical source identity.
    pub fn validate(&self) -> Result<(), String> {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                planning_request.validate()?;
                let inputs = planning_request.inputs();
                validate_bounded_text("planning attempt goal id", &inputs.goal_id, 256)?;
                if inputs.goal_updated_at_seq == 0 {
                    return Err(
                        "planning attempt goal update sequence must be positive".to_string()
                    );
                }
                inputs.projection_identity.validate()?;
                if inputs.projection_identity.goal_id() != inputs.goal_id
                    || inputs.projection_identity.goal_updated_at_seq()
                        != inputs.goal_updated_at_seq
                {
                    return Err(
                        "planning attempt goal identity does not match its projection".to_string(),
                    );
                }
                validate_digest("method library", &inputs.method_library_digest)?;
                validate_digest("capability catalog", &inputs.capability_catalog_digest)?;
                validate_bounded_text("planning version", &inputs.planning_version, 256)?;
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => {
                inputs.validate()?;
            }
        }
        if self.attempt_id != derive_attempt_id(self)? {
            return Err("planning attempt id does not match its immutable inputs".to_string());
        }
        Ok(())
    }

    /// Borrow the stable attempt id.
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    /// Borrow the frame-bound deterministic planning request when one exists.
    pub fn planning_request(&self) -> Option<&PlanningRequestIdentity> {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                Some(planning_request)
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { .. } => None,
        }
    }

    /// Borrow failed projection identity inputs when this attempt has no frame.
    pub fn projection_failure(&self) -> Option<&PlanningProjectionFailureIdentityInputs> {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { .. } => None,
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => Some(inputs),
        }
    }

    /// Borrow the selected goal id.
    pub fn goal_id(&self) -> &str {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                &planning_request.inputs().goal_id
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => &inputs.goal_id,
        }
    }

    /// Return the exact execution goal update sequence.
    pub fn goal_updated_at_seq(&self) -> u64 {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                planning_request.inputs().goal_updated_at_seq
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => {
                inputs.goal_updated_at_seq
            }
        }
    }

    /// Borrow the authoritative world-model projection frame id.
    pub fn projection_frame_id(&self) -> Option<&str> {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                Some(planning_request.inputs().projection_identity.frame_id())
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { .. } => None,
        }
    }

    /// Borrow the verified method library digest.
    pub fn method_library_digest(&self) -> &str {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                &planning_request.inputs().method_library_digest
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => {
                &inputs.method_library_digest
            }
        }
    }

    /// Borrow the capability catalog digest.
    pub fn capability_catalog_digest(&self) -> &str {
        match &self.source {
            PlanningAttemptSourceIdentity::ProjectionCompleted { planning_request } => {
                &planning_request.inputs().capability_catalog_digest
            }
            PlanningAttemptSourceIdentity::ProjectionFailed { inputs } => {
                &inputs.capability_catalog_digest
            }
        }
    }
}

impl TryFrom<PlanningAttemptIdentityWire> for PlanningAttemptIdentity {
    type Error = String;

    fn try_from(value: PlanningAttemptIdentityWire) -> Result<Self, Self::Error> {
        let identity = Self {
            attempt_id: value.attempt_id,
            source: value.source,
        };
        identity.validate()?;
        Ok(identity)
    }
}

/// Derive the stable task-network command id for one composed attempt.
pub fn planning_task_network_command_id(
    identity: &PlanningAttemptIdentity,
    composition_id: &str,
) -> Result<String, String> {
    identity.validate()?;
    if identity.planning_request().is_none() {
        return Err("projection-failure attempts cannot derive task-network commands".to_string());
    }
    if composition_id.trim().is_empty() {
        return Err("planning command composition id must be non-empty".to_string());
    }
    #[derive(Serialize)]
    struct CommandIdentity<'a> {
        attempt_id: &'a str,
        composition_id: &'a str,
    }
    let hash = hash_serializable(
        PLANNING_COMMAND_ID_DOMAIN,
        &CommandIdentity {
            attempt_id: identity.attempt_id(),
            composition_id,
        },
    )?;
    Ok(format!("execution-planning-task-network-command-{hash}"))
}

/// Terminal diagnostic classification for an attempt without a command receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningAttemptDiagnosticDisposition {
    /// The projection already satisfies the active goal.
    Satisfied,
    /// No verified method applies to the projected state.
    NoApplicableMethod,
    /// The projection lacks facts required for a deterministic decision.
    Indeterminate,
    /// Method verification prevented planning.
    InvalidMethod,
    /// Projection failed before execution planning could run.
    ProjectionFailed,
    /// Planning input or deterministic evaluation failed.
    PlanningFailed,
    /// Composition lowering failed before command preparation.
    LoweringFailed,
    /// Lowering completed without task-network mutations.
    NoMutation,
}

/// Bounded terminal diagnostic retained for audit and operator inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningAttemptTerminalDiagnostic {
    /// Stable terminal classification.
    pub disposition: PlanningAttemptDiagnosticDisposition,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Bounded human-readable diagnostic summary.
    pub message: String,
}

impl PlanningAttemptTerminalDiagnostic {
    /// Build and validate one terminal diagnostic.
    pub fn new(
        disposition: PlanningAttemptDiagnosticDisposition,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<Self, String> {
        let diagnostic = Self {
            disposition,
            code: code.into(),
            message: message.into(),
        };
        diagnostic.validate()?;
        Ok(diagnostic)
    }

    /// Validate bounded diagnostic text.
    pub fn validate(&self) -> Result<(), String> {
        validate_bounded_text(
            "planning diagnostic code",
            &self.code,
            MAX_DIAGNOSTIC_CODE_BYTES,
        )?;
        validate_bounded_text(
            "planning diagnostic message",
            &self.message,
            MAX_DIAGNOSTIC_MESSAGE_BYTES,
        )
    }
}

/// Stable summary of the deterministic result retained by an attempt audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningAttemptResultSummary {
    /// The projection already satisfied the selected goal.
    Satisfied,
    /// No verified method was applicable.
    NoApplicableMethod,
    /// The projected state could not decide the goal.
    Indeterminate,
    /// Method verification prevented deterministic planning.
    InvalidMethod,
    /// Projection failed before planning could run.
    ProjectionFailed,
    /// Planning input or deterministic evaluation failed.
    PlanningFailed,
    /// Planning selected and lowered one composition for command preparation.
    Composed {
        /// Stable selected composition id.
        composition_id: String,
    },
    /// Lowering failed after one method and composition were selected.
    LoweringFailed {
        /// Stable selected composition id.
        composition_id: String,
    },
    /// Lowering completed without task-network mutations.
    NoMutation {
        /// Stable selected composition id.
        composition_id: String,
    },
}

impl PlanningAttemptResultSummary {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Composed { composition_id }
            | Self::LoweringFailed { composition_id }
            | Self::NoMutation { composition_id } => {
                validate_bounded_text("planning composition id", composition_id, 256)
            }
            Self::Satisfied
            | Self::NoApplicableMethod
            | Self::Indeterminate
            | Self::InvalidMethod
            | Self::ProjectionFailed
            | Self::PlanningFailed => Ok(()),
        }
    }

    pub(crate) fn terminal_disposition(&self) -> Option<PlanningAttemptDiagnosticDisposition> {
        match self {
            Self::Satisfied => Some(PlanningAttemptDiagnosticDisposition::Satisfied),
            Self::NoApplicableMethod => {
                Some(PlanningAttemptDiagnosticDisposition::NoApplicableMethod)
            }
            Self::Indeterminate => Some(PlanningAttemptDiagnosticDisposition::Indeterminate),
            Self::InvalidMethod => Some(PlanningAttemptDiagnosticDisposition::InvalidMethod),
            Self::ProjectionFailed => Some(PlanningAttemptDiagnosticDisposition::ProjectionFailed),
            Self::PlanningFailed => Some(PlanningAttemptDiagnosticDisposition::PlanningFailed),
            Self::LoweringFailed { .. } => {
                Some(PlanningAttemptDiagnosticDisposition::LoweringFailed)
            }
            Self::NoMutation { .. } => Some(PlanningAttemptDiagnosticDisposition::NoMutation),
            Self::Composed { .. } => None,
        }
    }

    fn requires_selected_method(&self) -> bool {
        matches!(
            self,
            Self::Composed { .. } | Self::LoweringFailed { .. } | Self::NoMutation { .. }
        )
    }

    pub(crate) fn composition_id(&self) -> Option<&str> {
        match self {
            Self::Composed { composition_id }
            | Self::LoweringFailed { composition_id }
            | Self::NoMutation { composition_id } => Some(composition_id),
            Self::Satisfied
            | Self::NoApplicableMethod
            | Self::Indeterminate
            | Self::InvalidMethod
            | Self::ProjectionFailed
            | Self::PlanningFailed => None,
        }
    }
}

/// Versioned bounded diagnostic record for one deterministic planning decision.
///
/// This payload explains method selection and lowering. It never represents an
/// accepted execution plan, which remains owned by the task-network journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PlanningAttemptDecisionAuditWire")]
pub struct PlanningAttemptDecisionAudit {
    schema_version: u32,
    selected_method_id: Option<String>,
    candidate_reports: Vec<MethodCandidateReport>,
    projection_warnings: Vec<String>,
    method_diagnostics: Vec<PlanningDiagnostic>,
    lowering_diagnostics: Vec<LoweringDiagnostic>,
    result_summary: PlanningAttemptResultSummary,
    decision_digest: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PlanningAttemptDecisionAuditWire {
    schema_version: u32,
    selected_method_id: Option<String>,
    candidate_reports: Vec<MethodCandidateReport>,
    projection_warnings: Vec<String>,
    method_diagnostics: Vec<PlanningDiagnostic>,
    lowering_diagnostics: Vec<LoweringDiagnostic>,
    result_summary: PlanningAttemptResultSummary,
    decision_digest: String,
}

impl PlanningAttemptDecisionAudit {
    /// Build one canonical bounded decision audit and derive its stable digest.
    pub fn new(
        selected_method_id: Option<String>,
        candidate_reports: Vec<MethodCandidateReport>,
        mut projection_warnings: Vec<String>,
        method_diagnostics: Vec<PlanningDiagnostic>,
        lowering_diagnostics: Vec<LoweringDiagnostic>,
        result_summary: PlanningAttemptResultSummary,
    ) -> Result<Self, String> {
        projection_warnings.sort();
        projection_warnings.dedup();
        let mut audit = Self {
            schema_version: PLANNING_ATTEMPT_DECISION_SCHEMA_VERSION,
            selected_method_id,
            candidate_reports,
            projection_warnings,
            method_diagnostics,
            lowering_diagnostics,
            result_summary,
            decision_digest: String::new(),
        };
        audit.decision_digest = audit.recompute_digest()?;
        audit.validate()?;
        Ok(audit)
    }

    /// Validate schema, bounds, canonical warning order, and stable digest.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PLANNING_ATTEMPT_DECISION_SCHEMA_VERSION {
            return Err("unsupported planning decision audit schema".to_string());
        }
        if let Some(method_id) = &self.selected_method_id {
            validate_bounded_text("selected planning method id", method_id, 256)?;
        }
        if self.result_summary.requires_selected_method() && self.selected_method_id.is_none() {
            return Err("composed planning audit must name the selected method".to_string());
        }
        self.result_summary.validate()?;
        for count in [
            self.candidate_reports.len(),
            self.projection_warnings.len(),
            self.method_diagnostics.len(),
            self.lowering_diagnostics.len(),
        ] {
            if count > MAX_AUDIT_ITEMS {
                return Err(format!(
                    "planning decision audit collections may contain at most {MAX_AUDIT_ITEMS} items"
                ));
            }
        }
        let mut canonical_warnings = self.projection_warnings.clone();
        canonical_warnings.sort();
        canonical_warnings.dedup();
        if canonical_warnings != self.projection_warnings {
            return Err("planning projection warnings must be sorted and unique".to_string());
        }
        for warning in &self.projection_warnings {
            validate_bounded_text(
                "planning projection warning",
                warning,
                MAX_DIAGNOSTIC_MESSAGE_BYTES,
            )?;
        }
        for candidate in &self.candidate_reports {
            validate_bounded_text("planning candidate method id", &candidate.method_id, 256)?;
            validate_planning_diagnostics(&candidate.diagnostics)?;
        }
        validate_planning_diagnostics(&self.method_diagnostics)?;
        for diagnostic in &self.lowering_diagnostics {
            validate_bounded_text(
                "planning lowering diagnostic message",
                &diagnostic.message,
                MAX_DIAGNOSTIC_MESSAGE_BYTES,
            )?;
            validate_optional_bounded_text(
                "planning lowering step id",
                diagnostic.step_id.as_deref(),
                256,
            )?;
            validate_optional_bounded_text(
                "planning lowering operator id",
                diagnostic.operator_id.as_deref(),
                256,
            )?;
        }
        if self.decision_digest != self.recompute_digest()? {
            return Err("planning decision digest does not match its content".to_string());
        }
        if serde_json::to_vec(self)
            .map_err(|error| error.to_string())?
            .len()
            > MAX_AUDIT_BYTES
        {
            return Err(format!(
                "planning decision audit may contain at most {MAX_AUDIT_BYTES} encoded bytes"
            ));
        }
        Ok(())
    }

    /// Borrow the selected method id when planning chose one.
    pub fn selected_method_id(&self) -> Option<&str> {
        self.selected_method_id.as_deref()
    }

    /// Borrow the deterministic per-method candidate reports.
    pub fn candidate_reports(&self) -> &[MethodCandidateReport] {
        &self.candidate_reports
    }

    /// Borrow the canonical projection warnings retained for audit.
    pub fn projection_warnings(&self) -> &[String] {
        &self.projection_warnings
    }

    /// Borrow method selection and verification diagnostics.
    pub fn method_diagnostics(&self) -> &[PlanningDiagnostic] {
        &self.method_diagnostics
    }

    /// Borrow deterministic lowering diagnostics.
    pub fn lowering_diagnostics(&self) -> &[LoweringDiagnostic] {
        &self.lowering_diagnostics
    }

    /// Borrow the stable result summary.
    pub fn result_summary(&self) -> &PlanningAttemptResultSummary {
        &self.result_summary
    }

    /// Borrow the stable digest of this complete audit payload.
    pub fn decision_digest(&self) -> &str {
        &self.decision_digest
    }

    fn recompute_digest(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct DecisionDigestInput<'a> {
            schema_version: u32,
            selected_method_id: &'a Option<String>,
            candidate_reports: &'a [MethodCandidateReport],
            projection_warnings: &'a [String],
            method_diagnostics: &'a [PlanningDiagnostic],
            lowering_diagnostics: &'a [LoweringDiagnostic],
            result_summary: &'a PlanningAttemptResultSummary,
        }
        hash_serializable(
            PLANNING_DECISION_HASH_DOMAIN,
            &DecisionDigestInput {
                schema_version: self.schema_version,
                selected_method_id: &self.selected_method_id,
                candidate_reports: &self.candidate_reports,
                projection_warnings: &self.projection_warnings,
                method_diagnostics: &self.method_diagnostics,
                lowering_diagnostics: &self.lowering_diagnostics,
                result_summary: &self.result_summary,
            },
        )
    }
}

impl TryFrom<PlanningAttemptDecisionAuditWire> for PlanningAttemptDecisionAudit {
    type Error = String;

    fn try_from(value: PlanningAttemptDecisionAuditWire) -> Result<Self, Self::Error> {
        let audit = Self {
            schema_version: value.schema_version,
            selected_method_id: value.selected_method_id,
            candidate_reports: value.candidate_reports,
            projection_warnings: value.projection_warnings,
            method_diagnostics: value.method_diagnostics,
            lowering_diagnostics: value.lowering_diagnostics,
            result_summary: value.result_summary,
            decision_digest: value.decision_digest,
        };
        audit.validate()?;
        Ok(audit)
    }
}

/// Exact task-network command durably prepared before submission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningPreparedCommand {
    /// Stable hash of the complete task-network request.
    pub request_hash: String,
    /// Exact request that recovery must query or replay.
    pub request: command::Request,
}

impl PlanningPreparedCommand {
    /// Bind one planning attempt to its lowered task-network command.
    pub fn bind(
        identity: &PlanningAttemptIdentity,
        request: command::Request,
    ) -> Result<Self, String> {
        validate_planning_command(identity, &request)?;
        Ok(Self {
            request_hash: command_request_hash(&request)?,
            request,
        })
    }

    /// Validate command identity, payload scope, and request hash.
    pub fn validate(&self, identity: &PlanningAttemptIdentity) -> Result<(), String> {
        validate_planning_command(identity, &self.request)?;
        if self.request_hash != command_request_hash(&self.request)? {
            return Err("prepared planning command hash does not match its request".to_string());
        }
        Ok(())
    }
}

/// Durable terminal task-network response for one prepared command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningAttemptCommandOutcome {
    /// Command id copied from the exact prepared request.
    pub command_id: String,
    /// Request hash copied from the exact prepared command.
    pub request_hash: String,
    /// Stable digest of the returned response.
    pub response_hash: String,
    /// Persisted task-network command response.
    pub response: command::Response,
}

impl PlanningAttemptCommandOutcome {
    /// Bind one authority-issued receipt to the exact prepared command.
    pub fn bind(
        prepared: &PlanningPreparedCommand,
        receipt: command::OutcomeReceipt,
    ) -> Result<Self, String> {
        if receipt.command_id() != prepared.request.command_id
            || receipt.request_hash() != prepared.request_hash
            || receipt.request() != &prepared.request
        {
            return Err(
                "task-network outcome receipt does not match the prepared command".to_string(),
            );
        }
        let response = canonical_command_response(receipt.response().clone());
        validate_response_for_request(&prepared.request, &response)?;
        let outcome = Self {
            command_id: prepared.request.command_id.clone(),
            request_hash: prepared.request_hash.clone(),
            response_hash: response_hash(&response)?,
            response,
        };
        outcome.validate(prepared)?;
        Ok(outcome)
    }

    /// Validate response binding to the exact prepared request.
    pub fn validate(&self, prepared: &PlanningPreparedCommand) -> Result<(), String> {
        if self.command_id != prepared.request.command_id
            || self.request_hash != prepared.request_hash
            || self.response_hash != response_hash(&self.response)?
        {
            return Err("planning command outcome does not match the prepared command".to_string());
        }
        if self.response != canonical_command_response(self.response.clone()) {
            return Err("planning command outcome response is not canonical".to_string());
        }
        validate_response_for_request(&prepared.request, &self.response)?;
        Ok(())
    }
}

/// Append-only lifecycle record payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanningAttemptRecordKind {
    /// Establishes the immutable attempt identity.
    Opened {
        /// Complete deterministic attempt identity.
        identity: PlanningAttemptIdentity,
    },
    /// Preserves the bounded non-authoritative planning decision audit.
    DecisionAudited {
        /// Exact planning result and diagnostic audit payload.
        decision: PlanningAttemptDecisionAudit,
    },
    /// Terminates the attempt without a task-network command.
    TerminalDiagnostic {
        /// Bounded terminal diagnostic.
        diagnostic: PlanningAttemptTerminalDiagnostic,
    },
    /// Establishes the durable barrier before command submission.
    CommandPrepared {
        /// Exact task-network command and its request hash.
        prepared: Box<PlanningPreparedCommand>,
    },
    /// Records the terminal response for the exact prepared command.
    CommandOutcome {
        /// Task-network response bound to the prepared command.
        outcome: PlanningAttemptCommandOutcome,
    },
}

/// One immutable hash-chained planning attempt lifecycle record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningAttemptRecord {
    /// Durable record schema version.
    pub schema_version: u32,
    /// Stable attempt id shared by the complete chain.
    pub attempt_id: String,
    /// One-based append ordinal owned by the execution store.
    pub ordinal: u32,
    /// Hash of the preceding record when this is not the first record.
    pub previous_record_hash: Option<String>,
    /// Lifecycle payload.
    pub kind: PlanningAttemptRecordKind,
    /// Stable content hash over every preceding field.
    pub record_hash: String,
}

impl PlanningAttemptRecord {
    pub(crate) fn opened(identity: PlanningAttemptIdentity) -> Result<Self, String> {
        Self::identified(
            identity.attempt_id().to_string(),
            1,
            None,
            PlanningAttemptRecordKind::Opened { identity },
        )
    }

    pub(crate) fn successor(
        head: &PlanningAttemptHead,
        kind: PlanningAttemptRecordKind,
    ) -> Result<Self, String> {
        Self::identified(
            head.identity.attempt_id().to_string(),
            head.record_count.saturating_add(1),
            Some(head.head_hash.clone()),
            kind,
        )
    }

    fn identified(
        attempt_id: String,
        ordinal: u32,
        previous_record_hash: Option<String>,
        kind: PlanningAttemptRecordKind,
    ) -> Result<Self, String> {
        let mut record = Self {
            schema_version: PLANNING_ATTEMPT_SCHEMA_VERSION,
            attempt_id,
            ordinal,
            previous_record_hash,
            kind,
            record_hash: String::new(),
        };
        record.record_hash = record.recompute_hash()?;
        record.validate_shape()?;
        Ok(record)
    }

    /// Validate schema, hash, ordinal, and payload-local invariants.
    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema_version != PLANNING_ATTEMPT_SCHEMA_VERSION {
            return Err("unsupported planning attempt record schema".to_string());
        }
        if self.attempt_id.trim().is_empty() || self.ordinal == 0 {
            return Err("planning attempt record identity must be non-empty".to_string());
        }
        match &self.kind {
            PlanningAttemptRecordKind::Opened { identity } => {
                identity.validate()?;
                if self.ordinal != 1
                    || self.previous_record_hash.is_some()
                    || self.attempt_id != identity.attempt_id()
                {
                    return Err("planning attempt opening record is malformed".to_string());
                }
            }
            PlanningAttemptRecordKind::DecisionAudited { decision } => {
                decision.validate()?;
                validate_successor_shape(self)?;
            }
            PlanningAttemptRecordKind::TerminalDiagnostic { diagnostic } => {
                diagnostic.validate()?;
                validate_successor_shape(self)?;
            }
            PlanningAttemptRecordKind::CommandPrepared { prepared } => {
                if prepared.request_hash.trim().is_empty() {
                    return Err("prepared command hash must be non-empty".to_string());
                }
                validate_successor_shape(self)?;
            }
            PlanningAttemptRecordKind::CommandOutcome { outcome } => {
                if outcome.command_id.trim().is_empty()
                    || outcome.request_hash.trim().is_empty()
                    || outcome.response_hash.trim().is_empty()
                {
                    return Err("planning command outcome identity must be non-empty".to_string());
                }
                validate_successor_shape(self)?;
            }
        }
        if self.record_hash != self.recompute_hash()? {
            return Err("planning attempt record hash does not match its content".to_string());
        }
        Ok(())
    }

    fn recompute_hash(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct RecordHashInput<'a> {
            schema_version: u32,
            attempt_id: &'a str,
            ordinal: u32,
            previous_record_hash: &'a Option<String>,
            kind: &'a PlanningAttemptRecordKind,
        }
        hash_serializable(
            PLANNING_RECORD_HASH_DOMAIN,
            &RecordHashInput {
                schema_version: self.schema_version,
                attempt_id: &self.attempt_id,
                ordinal: self.ordinal,
                previous_record_hash: &self.previous_record_hash,
                kind: &self.kind,
            },
        )
    }
}

/// Current derived lifecycle state for one append-only attempt chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningAttemptState {
    /// Planning started and has no durable terminal product yet.
    Open,
    /// The exact non-authoritative planning decision audit is durable.
    DecisionAudited,
    /// An exact command is durable and may be queried or replayed.
    CommandPrepared,
    /// A terminal task-network command response is durable.
    CommandCompleted,
    /// A terminal diagnostic ended the attempt without a command.
    DiagnosticTerminal,
}

/// Derived head for one validated planning attempt record chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningAttemptHead {
    /// Complete immutable identity established by the opening record.
    pub identity: PlanningAttemptIdentity,
    /// Derived lifecycle state.
    pub state: PlanningAttemptState,
    /// Number of immutable records in this chain.
    pub record_count: u32,
    /// Hash of the last immutable record.
    pub head_hash: String,
}

impl PlanningAttemptHead {
    pub(crate) fn from_opened(record: &PlanningAttemptRecord) -> Result<Self, String> {
        let PlanningAttemptRecordKind::Opened { identity } = &record.kind else {
            return Err("planning attempt head requires an opening record".to_string());
        };
        Ok(Self {
            identity: identity.clone(),
            state: PlanningAttemptState::Open,
            record_count: 1,
            head_hash: record.record_hash.clone(),
        })
    }

    pub(crate) fn advance(&self, record: &PlanningAttemptRecord) -> Result<Self, String> {
        if record.attempt_id != self.identity.attempt_id()
            || record.ordinal != self.record_count.saturating_add(1)
            || record.previous_record_hash.as_deref() != Some(self.head_hash.as_str())
        {
            return Err("planning attempt record does not extend its head".to_string());
        }
        let state = match (&self.state, &record.kind) {
            (PlanningAttemptState::Open, PlanningAttemptRecordKind::DecisionAudited { .. }) => {
                PlanningAttemptState::DecisionAudited
            }
            (
                PlanningAttemptState::DecisionAudited,
                PlanningAttemptRecordKind::TerminalDiagnostic { .. },
            ) => PlanningAttemptState::DiagnosticTerminal,
            (
                PlanningAttemptState::DecisionAudited,
                PlanningAttemptRecordKind::CommandPrepared { .. },
            ) => PlanningAttemptState::CommandPrepared,
            (
                PlanningAttemptState::CommandPrepared,
                PlanningAttemptRecordKind::CommandOutcome { .. },
            ) => PlanningAttemptState::CommandCompleted,
            _ => return Err("invalid planning attempt lifecycle transition".to_string()),
        };
        Ok(Self {
            identity: self.identity.clone(),
            state,
            record_count: record.ordinal,
            head_hash: record.record_hash.clone(),
        })
    }
}

/// Bounded immutable history page for one attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningAttemptHistory {
    /// Records after the requested ordinal in ascending append order.
    pub records: Vec<PlanningAttemptRecord>,
    /// True when at least one additional record exists.
    pub budget_exhausted: bool,
}

/// Selector family fenced into one deterministic continuation token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum PlanningAttemptContinuationScope {
    Goal { goal_hash: String },
    Recovery,
}

/// Tamper-evident continuation fence for one bounded attempt selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlanningAttemptContinuationWire")]
pub struct PlanningAttemptContinuation {
    scope: PlanningAttemptContinuationScope,
    after_key: String,
    continuation_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct PlanningAttemptContinuationWire {
    scope: PlanningAttemptContinuationScope,
    after_key: String,
    continuation_hash: String,
}

impl PlanningAttemptContinuation {
    pub(crate) fn for_goal(goal_hash: String, after_key: String) -> Result<Self, String> {
        Self::identified(
            PlanningAttemptContinuationScope::Goal { goal_hash },
            after_key,
        )
    }

    pub(crate) fn for_recovery(after_key: String) -> Result<Self, String> {
        Self::identified(PlanningAttemptContinuationScope::Recovery, after_key)
    }

    fn identified(
        scope: PlanningAttemptContinuationScope,
        after_key: String,
    ) -> Result<Self, String> {
        let mut continuation = Self {
            scope,
            after_key,
            continuation_hash: String::new(),
        };
        continuation.continuation_hash = continuation.recompute_hash()?;
        continuation.validate()?;
        Ok(continuation)
    }

    pub(crate) fn validate_goal(&self, goal_hash: &str, prefix: &str) -> Result<&str, String> {
        self.validate()?;
        match &self.scope {
            PlanningAttemptContinuationScope::Goal { goal_hash: durable }
                if durable == goal_hash && self.after_key.starts_with(prefix) =>
            {
                Ok(&self.after_key)
            }
            _ => Err("planning attempt continuation does not match the goal selector".to_string()),
        }
    }

    pub(crate) fn validate_recovery(&self) -> Result<&str, String> {
        self.validate()?;
        if self.scope == PlanningAttemptContinuationScope::Recovery
            && self.after_key.starts_with("prepared::")
        {
            Ok(&self.after_key)
        } else {
            Err("planning attempt continuation does not match recovery selection".to_string())
        }
    }

    /// Validate selector scope, key bounds, and token integrity.
    pub fn validate(&self) -> Result<(), String> {
        validate_bounded_text("planning continuation key", &self.after_key, 1024)?;
        match &self.scope {
            PlanningAttemptContinuationScope::Goal { goal_hash } => {
                validate_digest("planning continuation goal", goal_hash)?;
            }
            PlanningAttemptContinuationScope::Recovery => {}
        }
        if self.continuation_hash != self.recompute_hash()? {
            return Err("planning attempt continuation hash mismatch".to_string());
        }
        Ok(())
    }

    fn recompute_hash(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct ContinuationHashInput<'a> {
            scope: &'a PlanningAttemptContinuationScope,
            after_key: &'a str,
        }
        hash_serializable(
            PLANNING_CONTINUATION_HASH_DOMAIN,
            &ContinuationHashInput {
                scope: &self.scope,
                after_key: &self.after_key,
            },
        )
    }
}

impl TryFrom<PlanningAttemptContinuationWire> for PlanningAttemptContinuation {
    type Error = String;

    fn try_from(value: PlanningAttemptContinuationWire) -> Result<Self, Self::Error> {
        let continuation = Self {
            scope: value.scope,
            after_key: value.after_key,
            continuation_hash: value.continuation_hash,
        };
        continuation.validate()?;
        Ok(continuation)
    }
}

/// Bounded deterministic attempt head selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningAttemptSelection {
    /// Attempt heads in deterministic goal sequence and identity order.
    pub attempts: Vec<PlanningAttemptHead>,
    /// True when at least one additional attempt exists.
    pub budget_exhausted: bool,
    /// Fence that resumes strictly after the final returned attempt.
    pub continuation: Option<PlanningAttemptContinuation>,
}

/// Prepared command selected for exact recovery after restart.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningAttemptRecovery {
    /// Complete attempt head at prepared-command state.
    pub head: PlanningAttemptHead,
    /// Exact command that recovery must query or replay.
    pub prepared: PlanningPreparedCommand,
}

/// Bounded deterministic recovery selection with continuation support.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningAttemptRecoverySelection {
    /// Exact prepared commands selected for recovery.
    pub recoveries: Vec<PlanningAttemptRecovery>,
    /// True when at least one additional recovery entry exists.
    pub budget_exhausted: bool,
    /// Fence that resumes strictly after the final returned recovery entry.
    pub continuation: Option<PlanningAttemptContinuation>,
}

pub(crate) fn command_request_hash(request: &command::Request) -> Result<String, String> {
    Ok(command::request_hash(request))
}

fn response_hash(response: &command::Response) -> Result<String, String> {
    hash_serializable(PLANNING_RESPONSE_HASH_DOMAIN, response)
}

fn canonical_command_response(response: command::Response) -> command::Response {
    match response {
        command::Response::Accepted {
            revision,
            state_hash,
        }
        | command::Response::Duplicate {
            revision,
            state_hash,
        } => command::Response::Accepted {
            revision,
            state_hash,
        },
        command::Response::Rejected(rejection) => command::Response::Rejected(rejection),
    }
}

fn validate_response_for_request(
    request: &command::Request,
    response: &command::Response,
) -> Result<(), String> {
    match response {
        command::Response::Accepted {
            revision,
            state_hash,
        } => {
            if request.base_revision.checked_add(1) != Some(*revision)
                || state_hash.trim().is_empty()
            {
                return Err("accepted planning command response is malformed".to_string());
            }
            validate_digest("accepted task-network state", state_hash)?;
        }
        command::Response::Duplicate { .. } => {
            return Err("duplicate planning command response must be canonicalized".to_string());
        }
        command::Response::Rejected(rejection) => match rejection {
            crate::task_network::mutation::Rejection::StaleBase { expected, .. }
                if *expected != request.base_revision =>
            {
                return Err("stale-base response does not match the prepared request".to_string());
            }
            crate::task_network::mutation::Rejection::StateHashMismatch { expected, .. }
                if expected != &request.base_state_hash =>
            {
                return Err("state-hash response does not match the prepared request".to_string());
            }
            crate::task_network::mutation::Rejection::DuplicateCommand(command_id)
                if command_id != &request.command_id =>
            {
                return Err(
                    "duplicate-command response does not match the prepared request".to_string(),
                );
            }
            _ => {}
        },
    }
    Ok(())
}

fn derive_attempt_id(identity: &PlanningAttemptIdentity) -> Result<String, String> {
    #[derive(Serialize)]
    struct AttemptIdentity<'a> {
        source: &'a PlanningAttemptSourceIdentity,
    }
    let hash = hash_serializable(
        PLANNING_ATTEMPT_ID_DOMAIN,
        &AttemptIdentity {
            source: &identity.source,
        },
    )?;
    Ok(format!("planning-attempt-{hash}"))
}

fn validate_planning_command(
    identity: &PlanningAttemptIdentity,
    request: &command::Request,
) -> Result<(), String> {
    identity.validate()?;
    validate_bounded_text("task-network command id", &request.command_id, 256)?;
    validate_bounded_text("task-network network id", &request.network_id, 256)?;
    validate_bounded_text(
        "task-network base state hash",
        &request.base_state_hash,
        256,
    )?;
    let command::Command::ApplyMutationSet(set) = &request.command else {
        return Err(
            "planning attempts may prepare only task-network mutation commands".to_string(),
        );
    };
    if set.network_id != request.network_id {
        return Err("planning command network does not match its mutation set".to_string());
    }
    let expected = planning_task_network_command_id(identity, &set.source_composition_id)?;
    if request.command_id != expected {
        return Err(
            "planning task-network command id does not match attempt and composition".to_string(),
        );
    }
    Ok(())
}

fn validate_successor_shape(record: &PlanningAttemptRecord) -> Result<(), String> {
    if record.ordinal <= 1
        || record
            .previous_record_hash
            .as_deref()
            .unwrap_or_default()
            .is_empty()
    {
        return Err("planning attempt successor record is malformed".to_string());
    }
    Ok(())
}

fn validate_digest(label: &str, digest: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(format!("{label} digest must be lowercase BLAKE3 hex"));
    }
    Ok(())
}

fn validate_bounded_text(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(format!("{label} must contain at most {max_bytes} bytes"));
    }
    Ok(())
}

fn validate_optional_bounded_text(
    label: &str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<(), String> {
    value.map_or(Ok(()), |value| {
        validate_bounded_text(label, value, max_bytes)
    })
}

fn validate_planning_diagnostics(diagnostics: &[PlanningDiagnostic]) -> Result<(), String> {
    if diagnostics.len() > MAX_AUDIT_ITEMS {
        return Err(format!(
            "planning diagnostic collections may contain at most {MAX_AUDIT_ITEMS} items"
        ));
    }
    for diagnostic in diagnostics {
        validate_bounded_text(
            "planning method diagnostic message",
            &diagnostic.message,
            MAX_DIAGNOSTIC_MESSAGE_BYTES,
        )?;
        validate_optional_bounded_text(
            "planning diagnostic method id",
            diagnostic.method_id.as_deref(),
            256,
        )?;
        validate_optional_bounded_text(
            "planning diagnostic step id",
            diagnostic.step_id.as_deref(),
            256,
        )?;
    }
    Ok(())
}

fn hash_serializable(domain: &[u8], value: &impl Serialize) -> Result<String, String> {
    let encoded = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}
