//! First-slice execution planning runtime.

use crate::capability::CapabilityCatalog;
use crate::planning::contracts::{
    CandidateStatus, ExecutionComposition, InvalidMethodReport, MethodCandidateReport,
    NoApplicableMethod, PlanningDiagnostic, PlanningDiagnosticCode, PlanningIndeterminate,
    PlanningInputError, PlanningRequest, PlanningResult, PlanningSatisfied,
};
use crate::planning::method_library::{operator_resolutions, MethodLibrary, VerifiedMethodEntry};
use meld_lang::{
    evaluate, substitute, validate, Bindings, Composition, CostEstimate, Effect, EvalResult,
    Operator, Proposition, Resolution, Step, StepKind, WorldState,
};
use serde::Serialize;

/// Runtime facade for one-goal execution planning.
#[derive(Debug, Clone)]
pub struct PlanningRuntime {
    method_library: MethodLibrary,
    capability_catalog: CapabilityCatalog,
}

impl PlanningRuntime {
    /// Create a planning runtime from a verified method library and catalog.
    pub fn new(method_library: MethodLibrary, capability_catalog: CapabilityCatalog) -> Self {
        Self {
            method_library,
            capability_catalog,
        }
    }

    /// Plan one active ground goal against one projected world state.
    pub fn plan_goal(
        &self,
        request: PlanningRequest,
    ) -> Result<PlanningResult, PlanningInputError> {
        validate_request(&request)?;

        if !self.method_library.invalid.is_empty() {
            return Ok(PlanningResult::InvalidMethod(
                invalid_method_library_report(&self.method_library.invalid),
            ));
        }

        match evaluate(&request.world_state, &request.goal.target) {
            EvalResult::Satisfied => {
                return Ok(PlanningResult::Satisfied(PlanningSatisfied {
                    goal: request.goal,
                    world_state_frame: request.world_state_frame,
                    diagnostics: vec![PlanningDiagnostic::new(
                        PlanningDiagnosticCode::GoalSatisfied,
                        "goal target already satisfies projected world state",
                    )],
                }));
            }
            EvalResult::Indeterminate { missing } => {
                return Ok(PlanningResult::Indeterminate(PlanningIndeterminate {
                    goal: request.goal,
                    world_state_frame: request.world_state_frame,
                    missing,
                    diagnostics: vec![PlanningDiagnostic::new(
                        PlanningDiagnosticCode::GoalIndeterminate,
                        "goal target cannot be evaluated from projected world state",
                    )],
                }));
            }
            EvalResult::Unsatisfied { .. } => {}
        }

        let mut candidate_reports = Vec::new();
        for entry in self.method_library.sorted_verified_entries() {
            match evaluate_candidate(entry, &request.world_state, &request.goal) {
                CandidateEvaluation::Rejected(report) => candidate_reports.push(report),
                CandidateEvaluation::Applicable { report, bindings } => {
                    candidate_reports.push(report);
                    return Ok(prepare_composition(
                        &request,
                        entry,
                        bindings,
                        candidate_reports,
                        &self.capability_catalog,
                    ));
                }
            }
        }

        Ok(PlanningResult::NoApplicableMethod(NoApplicableMethod {
            goal: request.goal,
            world_state_frame: request.world_state_frame,
            candidates: candidate_reports,
            diagnostics: vec![PlanningDiagnostic::new(
                PlanningDiagnosticCode::NoApplicableMethod,
                "no verified method applies to the active goal",
            )],
        }))
    }
}

enum CandidateEvaluation {
    Rejected(MethodCandidateReport),
    Applicable {
        report: MethodCandidateReport,
        bindings: Bindings,
    },
}

fn validate_request(request: &PlanningRequest) -> Result<(), PlanningInputError> {
    if request.request_id.trim().is_empty() {
        return Err(PlanningInputError::MissingRequestId);
    }
    if !matches!(request.goal.lifecycle, meld_lang::GoalLifecycle::Active) {
        return Err(PlanningInputError::NonActiveGoal {
            goal_id: request.goal.goal_id.clone(),
        });
    }
    if let Some(variable) = request.goal.target.grounding_issue() {
        return Err(PlanningInputError::NonGroundGoal {
            goal_id: request.goal.goal_id.clone(),
            variable,
        });
    }
    if request.world_state_request.goal_id != request.goal.goal_id {
        return Err(PlanningInputError::MismatchedWorldStateRequestGoal {
            request_goal_id: request.world_state_request.goal_id.clone(),
            goal_id: request.goal.goal_id.clone(),
        });
    }
    Ok(())
}

fn invalid_method_library_report(invalid: &[InvalidMethodReport]) -> InvalidMethodReport {
    let mut diagnostics = vec![PlanningDiagnostic::new(
        PlanningDiagnosticCode::MethodLibraryInvalid,
        "method library contains invalid method entries",
    )];
    diagnostics.extend(invalid.iter().flat_map(|report| report.diagnostics.clone()));
    InvalidMethodReport {
        source_ref: invalid.first().and_then(|report| report.source_ref.clone()),
        method_id: invalid.first().and_then(|report| report.method_id.clone()),
        diagnostics,
    }
}

fn evaluate_candidate(
    entry: &VerifiedMethodEntry,
    world_state: &WorldState,
    goal: &meld_lang::Goal,
) -> CandidateEvaluation {
    let method = &entry.method;
    let Some(bindings) = meld_lang::unify(&method.trigger, &goal.target) else {
        return CandidateEvaluation::Rejected(candidate_report(
            method,
            CandidateStatus::TriggerMiss,
            None,
            vec![candidate_diagnostic(
                PlanningDiagnosticCode::MethodTriggerMiss,
                "method trigger did not unify with goal target",
                method.method_id.as_str(),
            )],
        ));
    };

    for precondition in &method.preconditions {
        let substituted = match substitute_proposition(precondition, &bindings) {
            Ok(proposition) => proposition,
            Err(diagnostic) => {
                return CandidateEvaluation::Rejected(candidate_report(
                    method,
                    CandidateStatus::PreconditionsUnsatisfied,
                    Some(bindings),
                    vec![diagnostic.with_method(method.method_id.clone())],
                ));
            }
        };
        match evaluate(world_state, &substituted) {
            EvalResult::Satisfied => {}
            EvalResult::Unsatisfied { .. } => {
                return CandidateEvaluation::Rejected(candidate_report(
                    method,
                    CandidateStatus::PreconditionsUnsatisfied,
                    Some(bindings),
                    vec![candidate_diagnostic(
                        PlanningDiagnosticCode::MethodPreconditionUnsatisfied,
                        "method precondition is not satisfied by projected world state",
                        method.method_id.as_str(),
                    )],
                ));
            }
            EvalResult::Indeterminate { .. } => {
                return CandidateEvaluation::Rejected(candidate_report(
                    method,
                    CandidateStatus::PreconditionsIndeterminate,
                    Some(bindings),
                    vec![candidate_diagnostic(
                        PlanningDiagnosticCode::MethodPreconditionIndeterminate,
                        "method precondition cannot be evaluated from projected world state",
                        method.method_id.as_str(),
                    )],
                ));
            }
        }
    }

    if let Some(ceiling) = &goal.priority.cost_ceiling {
        if method.cost.exceeds(ceiling) {
            return CandidateEvaluation::Rejected(candidate_report(
                method,
                CandidateStatus::CostRejected,
                Some(bindings),
                vec![candidate_diagnostic(
                    PlanningDiagnosticCode::MethodCostCeilingExceeded,
                    "method cost exceeds the goal cost ceiling",
                    method.method_id.as_str(),
                )],
            ));
        }
    }

    let projected_effects = match substitute_effects(&method.net_effects, &bindings) {
        Ok(effects) => effects,
        Err(diagnostic) => {
            return CandidateEvaluation::Rejected(candidate_report(
                method,
                CandidateStatus::EffectProjectionFailed,
                Some(bindings),
                vec![diagnostic.with_method(method.method_id.clone())],
            ));
        }
    };
    let projected = match world_state.apply(&projected_effects) {
        Ok(projected) => projected,
        Err(error) => {
            return CandidateEvaluation::Rejected(candidate_report(
                method,
                CandidateStatus::EffectProjectionFailed,
                Some(bindings),
                vec![candidate_diagnostic(
                    PlanningDiagnosticCode::MethodEffectProjectionFailed,
                    format!("method net effects cannot be applied: {error:?}"),
                    method.method_id.as_str(),
                )],
            ));
        }
    };
    if evaluate(&projected, &goal.target) != EvalResult::Satisfied {
        return CandidateEvaluation::Rejected(candidate_report(
            method,
            CandidateStatus::EffectMiss,
            Some(bindings),
            vec![candidate_diagnostic(
                PlanningDiagnosticCode::MethodEffectMiss,
                "method projected effects do not satisfy the goal target",
                method.method_id.as_str(),
            )],
        ));
    }

    CandidateEvaluation::Applicable {
        report: candidate_report(
            method,
            CandidateStatus::Applicable,
            Some(bindings.clone()),
            vec![],
        ),
        bindings,
    }
}

fn prepare_composition(
    request: &PlanningRequest,
    entry: &VerifiedMethodEntry,
    bindings: Bindings,
    candidate_reports: Vec<MethodCandidateReport>,
    catalog: &CapabilityCatalog,
) -> PlanningResult {
    let projected_effects = match substitute_effects(&entry.method.net_effects, &bindings) {
        Ok(effects) => effects,
        Err(diagnostic) => {
            return PlanningResult::InvalidMethod(InvalidMethodReport {
                source_ref: Some(entry.source_ref.stable_ref()),
                method_id: Some(entry.method.method_id.clone()),
                diagnostics: vec![diagnostic.with_method(entry.method.method_id.clone())],
            });
        }
    };
    let composition = match substitute(&entry.method.composition, &bindings) {
        Ok(composition) => composition,
        Err(error) => {
            return PlanningResult::InvalidMethod(InvalidMethodReport {
                source_ref: Some(entry.source_ref.stable_ref()),
                method_id: Some(entry.method.method_id.clone()),
                diagnostics: error
                    .unbound_variables
                    .into_iter()
                    .map(|variable| {
                        PlanningDiagnostic::new(
                            PlanningDiagnosticCode::CompositionSubstitutionFailed,
                            format!("composition variable {} is unbound", variable.variable),
                        )
                        .with_method(entry.method.method_id.clone())
                        .with_step(variable.step_id)
                    })
                    .collect(),
            });
        }
    };
    let validation = validate(&composition);
    if !validation.valid {
        return PlanningResult::InvalidMethod(InvalidMethodReport {
            source_ref: Some(entry.source_ref.stable_ref()),
            method_id: Some(entry.method.method_id.clone()),
            diagnostics: validation
                .errors
                .iter()
                .map(|error| {
                    PlanningDiagnostic::new(
                        PlanningDiagnosticCode::CompositionValidationFailed,
                        format!("concrete composition is invalid: {error:?}"),
                    )
                    .with_method(entry.method.method_id.clone())
                })
                .collect(),
        });
    }

    let operator_resolutions = operator_resolutions(&composition, catalog);
    let diagnostics = candidate_reports
        .iter()
        .flat_map(|candidate| candidate.diagnostics.clone())
        .chain(
            operator_resolutions
                .iter()
                .flat_map(|report| report.diagnostics.clone()),
        )
        .collect::<Vec<_>>();

    PlanningResult::Composed(ExecutionComposition {
        composition_id: composition_id(
            &request.request_id,
            &request.goal.goal_id,
            &entry.method.method_id,
            &bindings,
            &composition,
            &request.world_state_frame.frame_id,
        ),
        goal: request.goal.clone(),
        world_state_frame: request.world_state_frame.clone(),
        method_id: entry.method.method_id.clone(),
        bindings,
        composition,
        projected_effects,
        operator_resolutions,
        validation,
        diagnostics,
    })
}

fn substitute_proposition(
    proposition: &Proposition,
    bindings: &Bindings,
) -> Result<Proposition, PlanningDiagnostic> {
    let composition = Composition {
        steps: vec![Step {
            step_id: "precondition".to_string(),
            kind: StepKind::Goal(proposition.clone()),
        }],
        edges: vec![],
    };
    let substituted = substitute(&composition, bindings).map_err(|error| {
        PlanningDiagnostic::new(
            PlanningDiagnosticCode::CompositionSubstitutionFailed,
            format!("precondition substitution failed: {error:?}"),
        )
    })?;
    let StepKind::Goal(proposition) = substituted.steps[0].kind.clone() else {
        unreachable!("temporary composition contains a goal step");
    };
    Ok(proposition)
}

fn substitute_effects(
    effects: &[Effect],
    bindings: &Bindings,
) -> Result<Vec<Effect>, PlanningDiagnostic> {
    let composition = Composition {
        steps: vec![Step {
            step_id: "effects".to_string(),
            kind: StepKind::Op(Operator {
                operator_id: "effects".to_string(),
                preconditions: vec![],
                effects: effects.to_vec(),
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: vec![],
                    requires_outputs: vec![],
                    scope_kind: None,
                    tags: vec![],
                    specific: None,
                },
            }),
        }],
        edges: vec![],
    };
    let substituted = substitute(&composition, bindings).map_err(|error| {
        PlanningDiagnostic::new(
            PlanningDiagnosticCode::MethodEffectProjectionFailed,
            format!("effect substitution failed: {error:?}"),
        )
    })?;
    let StepKind::Op(operator) = substituted.steps[0].kind.clone() else {
        unreachable!("temporary composition contains an operator step");
    };
    Ok(operator.effects)
}

fn candidate_report(
    method: &meld_lang::Method,
    status: CandidateStatus,
    bindings: Option<Bindings>,
    diagnostics: Vec<PlanningDiagnostic>,
) -> MethodCandidateReport {
    MethodCandidateReport {
        method_id: method.method_id.clone(),
        status,
        bindings,
        diagnostics,
    }
}

fn candidate_diagnostic(
    code: PlanningDiagnosticCode,
    message: impl Into<String>,
    method_id: &str,
) -> PlanningDiagnostic {
    PlanningDiagnostic::new(code, message).with_method(method_id.to_string())
}

fn composition_id(
    request_id: &str,
    goal_id: &str,
    method_id: &str,
    bindings: &Bindings,
    composition: &Composition,
    frame_id: &str,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        request_id: &'a str,
        goal_id: &'a str,
        method_id: &'a str,
        bindings: &'a Bindings,
        composition: &'a Composition,
        frame_id: &'a str,
    }

    let bytes = serde_json::to_vec(&Identity {
        request_id,
        goal_id,
        method_id,
        bindings,
        composition,
        frame_id,
    })
    .expect("planning composition identity is serializable");
    format!(
        "execution-composition-{}",
        hex::encode(blake3::hash(&bytes).as_bytes())
    )
}
