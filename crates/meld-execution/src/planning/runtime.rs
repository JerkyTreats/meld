//! First-slice execution planning runtime.

use crate::capability::CapabilityCatalog;
use crate::goals::PersistentGoalSetStore;
use crate::planning::contracts::{
    CandidateStatus, ExecutionComposition, IdentifiedPlanningRequest, InvalidMethodReport,
    MethodCandidateReport, NoApplicableMethod, PlanningDiagnostic, PlanningDiagnosticCode,
    PlanningIndeterminate, PlanningInputError, PlanningRequest, PlanningRequestIdentityInputs,
    PlanningResult, PlanningSatisfied,
};
use crate::planning::lowering::{
    Lowerer as ExecutionCompositionLowerer, Plan as CompositionLoweringPlan,
    Request as CompositionLoweringRequest,
};
use crate::planning::method_library::{operator_resolutions, MethodLibrary, VerifiedMethodEntry};
use crate::planning::world_state::{
    PlanningPerspectiveRef, PlanningProjectionIdentityInputs, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use crate::task::TaskDefinitionCompiler;
use crate::task_network::{
    command, mutation::ReadPrecondition, store::SledTaskNetworkStore, Command as TaskNetworkCommand,
};
use meld_lang::{
    evaluate, substitute, validate, Bindings, Composition, CostEstimate, Effect, EvalResult,
    GoalLifecycle, Operator, Proposition, Resolution, Step, StepKind, WorldState,
};
use serde::Serialize;

const PLANNING_ACTOR_ID: &str = "execution.planning.runtime";
const PLANNING_IDENTITY_VERSION: &str = "execution.planning.v1";

/// Request for one bounded execution planning actor pass.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningRuntimeActorRequest {
    /// Target task network for any lowered mutation command.
    pub network_id: String,
    /// Complete projection perspective included in each world state request.
    pub perspective: PlanningPerspectiveRef,
    /// Projection branch to include in each world state request.
    pub branch_id: String,
    /// Dimensions the projection port should prioritize for each goal.
    pub requested_dimensions: Vec<String>,
    /// Extra preconditions the projection port may include in its frame.
    pub required_preconditions: Vec<Proposition>,
    /// Optional maximum number of active goals to process in this pass.
    pub limit: Option<usize>,
}

/// Goal-scoped world state returned by the injected planner projection port.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningWorldStateProjection {
    /// Projected world state consumed by `PlanningRuntime`.
    pub world_state: WorldState,
    /// Durable projection provenance that travels with the planning result.
    pub frame: PlanningWorldStateFrameRef,
}

/// Projection failure returned by a planning projection port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningProjectionError {
    /// Stable summary suitable for actor reporting.
    pub message: String,
    /// True when a later actor pass may succeed without operator action.
    pub retryable: bool,
}

impl PlanningProjectionError {
    /// Build a retryable projection error.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    /// Build a fatal projection error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

/// Injected boundary for planner world state projection.
pub trait PlanningProjectionPort {
    /// Project world state for one active execution goal.
    fn project(
        &mut self,
        request: PlanningWorldStateRequest,
    ) -> Result<PlanningWorldStateProjection, PlanningProjectionError>;
}

impl<F> PlanningProjectionPort for F
where
    F: FnMut(
        PlanningWorldStateRequest,
    ) -> Result<PlanningWorldStateProjection, PlanningProjectionError>,
{
    fn project(
        &mut self,
        request: PlanningWorldStateRequest,
    ) -> Result<PlanningWorldStateProjection, PlanningProjectionError> {
        self(request)
    }
}

/// Diagnostic issue emitted by one planning actor pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningRuntimeActorIssue {
    /// Goal id associated with the issue when known.
    pub goal_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Result for one active goal processed by the planning actor.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum PlanningRuntimeActorGoalResult {
    /// Projection failed before planning or task network mutation.
    ProjectionFailed {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Projection failure summary.
        error: String,
        /// True when a later actor pass may retry the projection.
        retryable: bool,
    },
    /// Planning request validation failed before a semantic result.
    PlanningFailed {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Deterministic planning input error.
        error: PlanningInputError,
    },
    /// Planning completed without producing task network work.
    Planned {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Non-composed planning result.
        result: PlanningResult,
    },
    /// Lowering failed before command submission.
    LoweringFailed {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Composition that could not be lowered.
        composition_id: String,
        /// Lowering failure summary.
        error: String,
    },
    /// Lowering produced diagnostics but no task network mutations to submit.
    Lowered {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Lowering plan retained for caller inspection.
        plan: CompositionLoweringPlan,
    },
    /// Lowered composition was submitted through the task network command store.
    Submitted {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Lowering plan submitted through the command boundary.
        plan: CompositionLoweringPlan,
        /// Deterministic task network command id.
        command_id: String,
        /// Persisted command response.
        response: command::Response,
    },
}

/// Report returned by one bounded planning actor pass.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningRuntimeActorReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Number of active goals read from execution goal storage.
    pub active_goal_count: usize,
    /// Task network revision before actor work began.
    pub input_revision: u64,
    /// Task network revision after actor work completed.
    pub output_revision: u64,
    /// Active goals attempted by this pass.
    pub attempted: usize,
    /// Task network commands accepted or replayed as committed.
    pub committed: usize,
    /// Retryable diagnostics observed during the pass.
    pub retryable_errors: Vec<PlanningRuntimeActorIssue>,
    /// Fatal diagnostics observed during the pass.
    pub fatal_errors: Vec<PlanningRuntimeActorIssue>,
    /// True when more active goals remain after the configured limit.
    pub budget_exhausted: bool,
    /// Per-goal results in deterministic goal id order.
    pub results: Vec<PlanningRuntimeActorGoalResult>,
}

/// Error that prevents a planning actor pass from producing a report.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PlanningRuntimeActorError {
    /// The actor request violated execution runtime invariants.
    #[error("planning runtime actor request is invalid: {0}")]
    InvalidRequest(String),
    /// Active goal storage failed before actor work could continue.
    #[error("planning runtime actor goal store failed: {0}")]
    GoalStore(String),
}

/// Bounded execution actor facade from active goals to task network commands.
pub struct PlanningRuntimeActor<C> {
    actor_id: String,
    runtime: PlanningRuntime,
    lowerer: ExecutionCompositionLowerer<C>,
}

impl<C> PlanningRuntimeActor<C>
where
    C: TaskDefinitionCompiler,
{
    /// Create an actor facade over an existing planner and composition lowerer.
    pub fn new(runtime: PlanningRuntime, lowerer: ExecutionCompositionLowerer<C>) -> Self {
        Self {
            actor_id: PLANNING_ACTOR_ID.to_string(),
            runtime,
            lowerer,
        }
    }

    /// Return the stable actor id used in reports.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Process a bounded set of active goals through projection, planning, and command submission.
    pub fn run_once<P>(
        &self,
        goals: &PersistentGoalSetStore,
        task_network: &mut SledTaskNetworkStore,
        projection: &mut P,
        request: PlanningRuntimeActorRequest,
    ) -> Result<PlanningRuntimeActorReport, PlanningRuntimeActorError>
    where
        P: PlanningProjectionPort,
    {
        validate_actor_request(&request)?;

        let mut active_goal_records = goals
            .goal_records()
            .map_err(|error| PlanningRuntimeActorError::GoalStore(error.to_string()))?;
        active_goal_records.retain(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active));
        active_goal_records.sort_by(|left, right| {
            left.goal
                .priority
                .urgency
                .cmp(&right.goal.priority.urgency)
                .then_with(|| left.goal.goal_id.cmp(&right.goal.goal_id))
        });
        let active_goal_count = active_goal_records.len();
        let budget_exhausted = request
            .limit
            .map(|limit| active_goal_count > limit)
            .unwrap_or(false);
        let goal_limit = request.limit.unwrap_or(active_goal_count);
        let input_revision = task_network.state().revision;
        let mut report = PlanningRuntimeActorReport {
            actor_id: self.actor_id.clone(),
            active_goal_count,
            input_revision,
            output_revision: input_revision,
            attempted: 0,
            committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted,
            results: Vec::new(),
        };

        for record in active_goal_records.into_iter().take(goal_limit) {
            report.attempted += 1;
            self.process_goal(
                task_network,
                projection,
                &request,
                record.goal,
                record.updated_at_seq,
                &mut report,
            );
        }

        report.output_revision = task_network.state().revision;
        Ok(report)
    }

    fn process_goal<P>(
        &self,
        task_network: &mut SledTaskNetworkStore,
        projection: &mut P,
        request: &PlanningRuntimeActorRequest,
        goal: meld_lang::Goal,
        goal_updated_at_seq: u64,
        report: &mut PlanningRuntimeActorReport,
    ) where
        P: PlanningProjectionPort,
    {
        let projection_request = projection_request_for_goal(request, &goal);
        let projected = match projection.project(projection_request.clone()) {
            Ok(projected) => projected,
            Err(error) => {
                let issue = PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_projection_failed".to_string(),
                    message: error.message.clone(),
                };
                if error.retryable {
                    report.retryable_errors.push(issue);
                } else {
                    report.fatal_errors.push(issue);
                }
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::ProjectionFailed {
                        goal_id: goal.goal_id,
                        error: error.message,
                        retryable: error.retryable,
                    });
                return;
            }
        };

        let projection_identity = match PlanningProjectionIdentityInputs::from_projection(
            &projection_request,
            &projected.world_state,
            &projected.frame,
        ) {
            Ok(identity) => identity,
            Err(error) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_projection_identity_failed".to_string(),
                    message: error.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::PlanningFailed {
                        goal_id: goal.goal_id,
                        error: PlanningInputError::IdentityMismatch { message: error },
                    });
                return;
            }
        };
        let planning_request = PlanningRequest {
            request_id: String::new(),
            goal: goal.clone(),
            world_state: projected.world_state,
            world_state_frame: projected.frame,
            world_state_request: projection_request,
        };
        let identified_request = match IdentifiedPlanningRequest::bind(
            planning_request,
            PlanningRequestIdentityInputs {
                goal_id: goal.goal_id.clone(),
                goal_updated_at_seq,
                projection_identity,
                method_library_digest: self.runtime.method_library.digest(),
                capability_catalog_digest: self.runtime.capability_catalog.digest(),
                planning_version: PLANNING_IDENTITY_VERSION.to_string(),
            },
        ) {
            Ok(request) => request,
            Err(error) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_identity_mismatch".to_string(),
                    message: error.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::PlanningFailed {
                        goal_id: goal.goal_id,
                        error: PlanningInputError::IdentityMismatch { message: error },
                    });
                return;
            }
        };
        let planning_result = match self.runtime.plan_goal(identified_request.request().clone()) {
            Ok(result) => result,
            Err(error) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_input_failed".to_string(),
                    message: format!("{error:?}"),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::PlanningFailed {
                        goal_id: goal.goal_id,
                        error,
                    });
                return;
            }
        };

        let PlanningResult::Composed(composition) = planning_result else {
            report
                .results
                .push(PlanningRuntimeActorGoalResult::Planned {
                    goal_id: goal.goal_id,
                    result: planning_result,
                });
            return;
        };

        let lower_request = CompositionLoweringRequest {
            request_id: lowering_request_id(&composition),
            network_id: request.network_id.clone(),
            idempotency_key: lowering_idempotency_key(&composition),
            composition,
        };
        let composition_id = lower_request.composition.composition_id.clone();
        let plan = match self.lowerer.lower(lower_request) {
            Ok(plan) => plan,
            Err(error) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "composition_lowering_failed".to_string(),
                    message: error.to_string(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                        goal_id: goal.goal_id,
                        composition_id,
                        error: error.to_string(),
                    });
                return;
            }
        };

        if plan.mutations.mutations.is_empty() {
            report
                .results
                .push(PlanningRuntimeActorGoalResult::Lowered {
                    goal_id: goal.goal_id,
                    plan,
                });
            return;
        }

        if plan_already_materialized(task_network.state(), &plan) {
            report
                .results
                .push(PlanningRuntimeActorGoalResult::Lowered {
                    goal_id: goal.goal_id,
                    plan,
                });
            return;
        }

        let base_revision = task_network.state().revision;
        let base_state_hash = task_network.state().state_hash.clone();
        let command_id = planning_command_id(&goal.goal_id, &plan, base_revision, &base_state_hash);
        let command_request = command::Request {
            command_id: command_id.clone(),
            network_id: plan.network_id.clone(),
            base_revision,
            base_state_hash: base_state_hash.clone(),
            read_preconditions: vec![
                ReadPrecondition::RevisionIs(base_revision),
                ReadPrecondition::StateHashIs(base_state_hash),
            ],
            command: TaskNetworkCommand::ApplyMutationSet(plan.mutations.clone()),
        };
        let response = match task_network.submit(command_request) {
            Ok(response) => response,
            Err(error) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "task_network_store_failed".to_string(),
                    message: error.to_string(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                        goal_id: goal.goal_id,
                        composition_id: plan.composition_id,
                        error: error.to_string(),
                    });
                return;
            }
        };
        if matches!(
            response,
            command::Response::Accepted { .. } | command::Response::Duplicate { .. }
        ) {
            report.committed += 1;
        } else if let command::Response::Rejected(rejection) = &response {
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "task_network_command_rejected".to_string(),
                message: format!("{rejection:?}"),
            });
        }
        report
            .results
            .push(PlanningRuntimeActorGoalResult::Submitted {
                goal_id: goal.goal_id,
                plan,
                command_id,
                response,
            });
    }
}

/// Runtime facade for one-goal execution planning.
#[derive(Debug, Clone)]
pub struct PlanningRuntime {
    method_library: MethodLibrary,
    capability_catalog: CapabilityCatalog,
}

fn validate_actor_request(
    request: &PlanningRuntimeActorRequest,
) -> Result<(), PlanningRuntimeActorError> {
    if request.network_id.trim().is_empty() {
        return Err(PlanningRuntimeActorError::InvalidRequest(
            "network_id must be non-empty".to_string(),
        ));
    }
    request
        .perspective
        .validate()
        .map_err(PlanningRuntimeActorError::InvalidRequest)?;
    if request.branch_id.trim().is_empty() {
        return Err(PlanningRuntimeActorError::InvalidRequest(
            "branch_id must be non-empty".to_string(),
        ));
    }
    Ok(())
}

fn projection_request_for_goal(
    request: &PlanningRuntimeActorRequest,
    goal: &meld_lang::Goal,
) -> PlanningWorldStateRequest {
    PlanningWorldStateRequest {
        goal_id: goal.goal_id.clone(),
        agent_id: goal.agent_id.clone(),
        target: goal.target.clone(),
        perspective: request.perspective.clone(),
        branch_id: request.branch_id.clone(),
        requested_dimensions: request.requested_dimensions.clone(),
        required_preconditions: request.required_preconditions.clone(),
    }
}

fn lowering_request_id(composition: &ExecutionComposition) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        goal_id: &'a str,
        composition_id: &'a str,
        frame_id: &'a str,
    }

    stable_runtime_id(
        "execution-planning-lowering-request",
        &Identity {
            goal_id: &composition.goal.goal_id,
            composition_id: &composition.composition_id,
            frame_id: &composition.world_state_frame.frame_id,
        },
    )
}

fn lowering_idempotency_key(composition: &ExecutionComposition) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        goal_id: &'a str,
        composition_id: &'a str,
        method_id: &'a str,
        frame_id: &'a str,
    }

    stable_runtime_id(
        "execution-planning-lowering",
        &Identity {
            goal_id: &composition.goal.goal_id,
            composition_id: &composition.composition_id,
            method_id: &composition.method_id,
            frame_id: &composition.world_state_frame.frame_id,
        },
    )
}

fn plan_already_materialized(
    state: &crate::task_network::state::NetworkState,
    plan: &CompositionLoweringPlan,
) -> bool {
    plan.mutations
        .mutations
        .iter()
        .all(|mutation| match mutation {
            crate::task_network::mutation::Mutation::Inject(inject) => {
                state.tasks.contains_key(&inject.task_node.task_instance_id)
            }
        })
}

fn planning_command_id(
    goal_id: &str,
    plan: &CompositionLoweringPlan,
    base_revision: u64,
    base_state_hash: &str,
) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        goal_id: &'a str,
        network_id: &'a str,
        composition_id: &'a str,
        mutation_set_id: &'a str,
        base_revision: u64,
        base_state_hash: &'a str,
    }

    stable_runtime_id(
        "execution-planning-task-network-command",
        &Identity {
            goal_id,
            network_id: &plan.network_id,
            composition_id: &plan.composition_id,
            mutation_set_id: &plan.mutations.set_id,
            base_revision,
            base_state_hash,
        },
    )
}

fn stable_runtime_id(prefix: &str, value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("planning runtime identity is serializable");
    format!("{}-{}", prefix, blake3::hash(&bytes).to_hex())
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
    /// Candidate failed one or more planning checks.
    Rejected(MethodCandidateReport),
    /// Candidate passed checks and has concrete bindings.
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
    PlanningProjectionIdentityInputs::from_projection(
        &request.world_state_request,
        &request.world_state,
        &request.world_state_frame,
    )
    .map_err(|message| PlanningInputError::IdentityMismatch { message })?;
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
