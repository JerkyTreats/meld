//! First-slice execution planning runtime.

use crate::capability::CapabilityCatalog;
use crate::goals::{
    ExecutionGoalRecord, GoalPlanningSelectionError, GoalPlanningSelectionPort,
    MAX_PLANNING_SELECTION_LIMIT,
};
use crate::planning::attempts::{
    planning_task_network_command_id, PlanningAttemptDecisionAudit,
    PlanningAttemptDiagnosticDisposition, PlanningAttemptIdentity, PlanningAttemptQuery,
    PlanningAttemptRecovery, PlanningAttemptResultSummary, PlanningAttemptStorageError,
    PlanningAttemptStore, PlanningAttemptTerminalDiagnostic, PlanningPreparedCommand,
    PlanningProjectionFailureIdentityInputs,
};
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
    command,
    mutation::{ReadPrecondition, Rejection},
    Command as TaskNetworkCommand, TaskNetworkAuthorityError, TaskNetworkAuthorityPorts,
    TaskNetworkMaterialization,
};
use meld_lang::{
    evaluate, substitute, validate, Bindings, Composition, CostEstimate, Effect, EvalResult,
    GoalLifecycle, Operator, Proposition, Resolution, Step, StepKind, WorldState,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;

const PLANNING_ACTOR_ID: &str = "execution.planning";
const PLANNING_IDENTITY_VERSION: &str = "execution.planning.v1";
const DEFAULT_PLANNING_GOAL_LIMIT: usize = 64;
const MAX_PLANNING_TASKS_PER_GOAL: usize = 1_024;
const MAX_PLANNING_INCOMING_EDGES_PER_TASK: usize = 1_024;
const MAX_PLANNING_TASK_ID_BYTES: usize = 1_024;
const MAX_PLANNING_COMMAND_BYTES: usize = 16 * 1_048_576;
const MAX_PLANNING_PROJECTION_ITEMS: usize = 4_096;
const MAX_PLANNING_TEXT_BYTES: usize = 4_096;
const MAX_PLANNING_PROJECTION_BYTES: usize = 16 * 1_048_576;
const MAX_PLANNING_TICK_PROJECTION_BYTES: usize = 32 * 1_048_576;
const MAX_PLANNING_TICK_RETAINED_PLAN_BYTES: usize = 32 * 1_048_576;
const MAX_PLANNING_TICK_PRODUCT_BYTES: usize = 48 * 1_048_576;
const MAX_PLANNING_COMMAND_ATTEMPTS: usize = 1_024;

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
    ///
    /// Omission uses the execution-owned bounded default.
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
    /// Task-network query or command authority could not complete the handoff.
    CommandFailed {
        /// Goal selected from execution goal storage.
        goal_id: String,
        /// Deterministic command id when command construction completed.
        command_id: Option<String>,
        /// Authority or command rejection summary.
        error: String,
        /// True when a later bounded pass may safely retry.
        retryable: bool,
    },
    /// Lowered composition was acknowledged by the task network authority.
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
    /// A previously prepared command reached a durable authority outcome.
    Recovered {
        /// Goal bound to the recovered planning attempt.
        goal_id: String,
        /// Stable planning attempt id.
        attempt_id: String,
        /// Exact prepared task network command id.
        command_id: String,
        /// Persisted command response authenticated by the authority query.
        response: command::Response,
    },
    /// Durable planning attempt authority rejected or could not persist progress.
    AttemptFailed {
        /// Goal id associated with the attempt when known.
        goal_id: Option<String>,
        /// Stable attempt id when identity construction completed.
        attempt_id: Option<String>,
        /// Durable authority failure summary.
        error: String,
        /// True when the exact operation may safely be retried.
        retryable: bool,
    },
}

/// Report returned by one bounded planning actor pass.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanningRuntimeActorReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Number of active goals recorded by the durable planning index.
    pub active_goal_count: usize,
    /// Task network revision before actor work began.
    pub input_revision: u64,
    /// Verified task network revision after actor work completed.
    ///
    /// This is absent when the final authority query fails, even if an earlier
    /// command acknowledgement proved partial progress during the pass.
    pub output_revision: Option<u64>,
    /// Highest task network revision durably acknowledged during this pass.
    ///
    /// Unlike `output_revision`, this remains available when the final query
    /// cannot prove the authority's closing revision.
    pub last_acknowledged_revision: u64,
    /// Active goals attempted by this pass.
    pub attempted: usize,
    /// Task network commands accepted or replayed as committed.
    pub committed: usize,
    /// Retryable diagnostics observed during the pass.
    pub retryable_errors: Vec<PlanningRuntimeActorIssue>,
    /// Fatal diagnostics observed during the pass.
    pub fatal_errors: Vec<PlanningRuntimeActorIssue>,
    /// True when recovery or active-goal work consumed the configured item budget.
    pub budget_exhausted: bool,
    /// Per-goal results in deterministic planning selection order.
    pub results: Vec<PlanningRuntimeActorGoalResult>,
}

/// Error that prevents a planning actor pass from producing a report.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum PlanningRuntimeActorError {
    /// The actor request violated execution runtime invariants.
    #[error("planning runtime actor request is invalid: {0}")]
    InvalidRequest(String),
    /// Active goal storage failed before actor work could continue.
    #[error("planning runtime actor goal store failed: {message}")]
    GoalStore {
        /// Stable storage diagnostic.
        message: String,
        /// True when a later read may recover without operator repair.
        retryable: bool,
    },
    /// The task-network authority could not provide the initial durable view.
    #[error("planning runtime actor task-network authority failed: {message}")]
    TaskNetworkAuthority {
        /// Stable authority diagnostic.
        message: String,
        /// True when supervisor replacement or later admission may recover.
        retryable: bool,
    },
    /// Durable planning attempt authority failed before a report could be produced.
    #[error("planning runtime actor attempt authority failed: {message}")]
    PlanningAttemptAuthority {
        /// Stable persistence diagnostic.
        message: String,
        /// True when a later bounded pass may safely retry.
        retryable: bool,
    },
}

impl PlanningRuntimeActorError {
    /// Return whether the actor pass may be retried without changing input.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::GoalStore {
                retryable: true,
                ..
            } | Self::TaskNetworkAuthority {
                retryable: true,
                ..
            } | Self::PlanningAttemptAuthority {
                retryable: true,
                ..
            }
        )
    }
}

/// Bounded execution actor facade from active goals to task network commands.
pub struct PlanningRuntimeActor<C> {
    actor_id: String,
    runtime: PlanningRuntime,
    lowerer: ExecutionCompositionLowerer<C>,
    attempt_store: Option<Arc<PlanningAttemptStore>>,
    bound_attempt_store: Option<Arc<PlanningAttemptStore>>,
}

impl<C> PlanningRuntimeActor<C>
where
    C: TaskDefinitionCompiler,
{
    /// Create the legacy actor facade without durable planning attempt authority.
    ///
    /// This compatibility path preserves callers that have not yet supplied the
    /// execution-owned attempt store. Production runtime assembly must use
    /// `new_durable` and bind a supervisor lease before ticking.
    pub fn new(runtime: PlanningRuntime, lowerer: ExecutionCompositionLowerer<C>) -> Self {
        Self {
            actor_id: PLANNING_ACTOR_ID.to_string(),
            runtime,
            lowerer,
            attempt_store: None,
            bound_attempt_store: None,
        }
    }

    /// Create the production actor with an unbound durable attempt authority.
    pub fn new_durable(
        runtime: PlanningRuntime,
        lowerer: ExecutionCompositionLowerer<C>,
        attempt_store: Arc<PlanningAttemptStore>,
    ) -> Self {
        Self {
            actor_id: PLANNING_ACTOR_ID.to_string(),
            runtime,
            lowerer,
            attempt_store: Some(attempt_store),
            bound_attempt_store: None,
        }
    }

    /// Bind planning writes to the current supervisor lease generation.
    pub fn bind_owner(
        &mut self,
        lease_id: impl Into<String>,
    ) -> Result<(), PlanningAttemptStorageError> {
        let store = self.attempt_store.as_ref().ok_or_else(|| {
            PlanningAttemptStorageError::InvalidInput(
                "compatibility planning actor has no durable attempt authority".to_string(),
            )
        })?;
        let expected = store.active_owner_fence()?;
        let bound = store.bind_owner(expected.as_ref(), self.actor_id.clone(), lease_id)?;
        self.bound_attempt_store = Some(Arc::new(bound));
        Ok(())
    }

    /// Drop the in-memory writer capability when the supervisor stops this actor.
    pub fn clear_owner_binding(&mut self) {
        self.bound_attempt_store = None;
    }

    /// Borrow the unbound store retained for factory cloning and shutdown flush.
    pub fn attempt_store(&self) -> Option<&Arc<PlanningAttemptStore>> {
        self.attempt_store.as_ref()
    }

    /// Return the stable actor id used in reports.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Process a bounded set of active goals through projection, planning, and command submission.
    pub fn run_once<G, P>(
        &self,
        goals: &G,
        task_network: &TaskNetworkAuthorityPorts,
        projection: &mut P,
        request: PlanningRuntimeActorRequest,
    ) -> Result<PlanningRuntimeActorReport, PlanningRuntimeActorError>
    where
        G: GoalPlanningSelectionPort + ?Sized,
        P: PlanningProjectionPort,
    {
        validate_actor_request(&request)?;
        let durable_attempt_store = match (&self.attempt_store, &self.bound_attempt_store) {
            (None, None) => None,
            (Some(_), Some(bound)) => {
                bound
                    .validate_owner_fence()
                    .map_err(planning_attempt_authority_error)?;
                Some(Arc::clone(bound))
            }
            (Some(_), None) => {
                return Err(PlanningRuntimeActorError::PlanningAttemptAuthority {
                    message: "durable planning actor must bind a supervisor lease before ticking"
                        .to_string(),
                    retryable: false,
                });
            }
            (None, Some(_)) => {
                return Err(PlanningRuntimeActorError::PlanningAttemptAuthority {
                    message: "planning actor retained a writer without its source authority"
                        .to_string(),
                    retryable: false,
                });
            }
        };
        let authority_identity = task_network.lifecycle();
        if authority_identity.network_id != request.network_id {
            return Err(PlanningRuntimeActorError::InvalidRequest(format!(
                "network_id '{}' does not match task network authority '{}'",
                request.network_id, authority_identity.network_id
            )));
        }

        let input_head = task_network
            .query()
            .head()
            .map_err(planning_authority_error)?;
        let input_revision = input_head.revision;
        let goal_limit = request.limit.unwrap_or(DEFAULT_PLANNING_GOAL_LIMIT);
        let mut pass = PlanningActorPass {
            report: PlanningRuntimeActorReport {
                actor_id: self.actor_id.clone(),
                active_goal_count: 0,
                input_revision,
                output_revision: Some(input_revision),
                last_acknowledged_revision: input_revision,
                attempted: 0,
                committed: 0,
                retryable_errors: Vec::new(),
                fatal_errors: Vec::new(),
                budget_exhausted: false,
                results: Vec::new(),
            },
            product_budget: PlanningTickProductBudget::default(),
        };

        let mut remaining = goal_limit;
        if let Some(store) = durable_attempt_store.as_ref() {
            let recovery = store
                .claim_recoverable_commands_bounded(goal_limit)
                .map_err(planning_attempt_authority_error)?;
            let recovered_count = recovery.recoveries.len();
            for prepared in recovery.recoveries {
                pass.report.attempted += 1;
                process_prepared_recovery(
                    store,
                    task_network,
                    &request,
                    prepared,
                    &mut pass.report,
                );
            }
            remaining = remaining.saturating_sub(recovered_count);
            pass.report.budget_exhausted = recovery.budget_exhausted || remaining == 0;
        }

        let mut claim = None;
        if remaining > 0 && !pass.report.budget_exhausted {
            let selection = goals
                .claim_active_goals(remaining)
                .map_err(planning_goal_store_error)?;
            pass.report.active_goal_count = selection.active_goal_count;
            pass.report.budget_exhausted |= selection.budget_exhausted;
            claim = selection.claim;
            for record in selection.records {
                pass.report.attempted += 1;
                self.process_goal(
                    goals,
                    task_network,
                    projection,
                    &request,
                    record,
                    durable_attempt_store.as_deref(),
                    &mut pass,
                );
            }
        }

        match task_network.query().head() {
            Ok(head) => {
                pass.report.last_acknowledged_revision =
                    pass.report.last_acknowledged_revision.max(head.revision);
                pass.report.output_revision = Some(head.revision);
            }
            Err(error) => {
                pass.report.output_revision = None;
                push_authority_issue(
                    &mut pass.report,
                    None,
                    "task_network_final_query_failed",
                    error,
                );
            }
        }
        if let Some(claim) = claim {
            if let Err(error) = goals.release_goal_claim(&claim) {
                let retryable = error.retryable();
                let issue = PlanningRuntimeActorIssue {
                    goal_id: None,
                    code: "planning_goal_claim_release_failed".to_string(),
                    message: error.to_string(),
                };
                if retryable {
                    pass.report.retryable_errors.push(issue);
                } else {
                    pass.report.fatal_errors.push(issue);
                }
            }
        }
        Ok(pass.report)
    }

    #[allow(clippy::too_many_arguments)]
    fn process_goal<G, P>(
        &self,
        goals: &G,
        task_network: &TaskNetworkAuthorityPorts,
        projection: &mut P,
        request: &PlanningRuntimeActorRequest,
        record: ExecutionGoalRecord,
        attempt_store: Option<&PlanningAttemptStore>,
        pass: &mut PlanningActorPass,
    ) where
        G: GoalPlanningSelectionPort + ?Sized,
        P: PlanningProjectionPort,
    {
        let report = &mut pass.report;
        let product_budget = &mut pass.product_budget;
        let goal_updated_at_seq = record.updated_at_seq;
        let goal = record.goal;
        let projection_request =
            match projection_request_for_goal(request, &goal, goal_updated_at_seq) {
                Ok(projection_request) => projection_request,
                Err(error) => {
                    report.fatal_errors.push(PlanningRuntimeActorIssue {
                        goal_id: Some(goal.goal_id.clone()),
                        code: "planning_projection_request_invalid".to_string(),
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
        let projected = match projection.project(projection_request.clone()) {
            Ok(projected) => projected,
            Err(error) => {
                if !error.retryable {
                    if let Some(store) = attempt_store {
                        self.persist_projection_failure(
                            store,
                            &projection_request,
                            &error.message,
                            report,
                        );
                    }
                }
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
        let projection_bytes = match validate_projection_bounds(&projected) {
            Ok(bytes) => bytes,
            Err(message) => {
                if let Some(store) = attempt_store {
                    self.persist_projection_failure(store, &projection_request, &message, report);
                }
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_projection_bounds_exceeded".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::ProjectionFailed {
                        goal_id: goal.goal_id,
                        error: message,
                        retryable: false,
                    });
                return;
            }
        };
        if let Err(message) = product_budget.reserve_projection(projection_bytes) {
            if let Some(store) = attempt_store {
                self.persist_projection_failure(store, &projection_request, &message, report);
            }
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "planning_tick_projection_budget_exhausted".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::ProjectionFailed {
                    goal_id: goal.goal_id,
                    error: message,
                    retryable: false,
                });
            return;
        }

        let projection_identity = match PlanningProjectionIdentityInputs::from_projection(
            &projection_request,
            &projected.world_state,
            &projected.frame,
        ) {
            Ok(identity) => identity,
            Err(error) => {
                if let Some(store) = attempt_store {
                    self.persist_projection_failure(store, &projection_request, &error, report);
                }
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
        if let Some(store) = attempt_store {
            self.process_identified_goal_durable(
                goals,
                task_network,
                request,
                goal,
                goal_updated_at_seq,
                identified_request,
                store,
                pass,
            );
            return;
        }
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

        let retained_plan_bytes = match validate_planning_plan_bounds(&plan) {
            Ok(bytes) => bytes,
            Err(message) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_plan_bounds_exceeded".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                        goal_id: goal.goal_id,
                        composition_id,
                        error: message,
                    });
                return;
            }
        };
        if let Err(message) = product_budget.reserve_plan(retained_plan_bytes) {
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "planning_tick_plan_budget_exhausted".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                    goal_id: goal.goal_id,
                    composition_id,
                    error: message,
                });
            return;
        }

        if plan.mutations.mutations.is_empty() {
            report
                .results
                .push(PlanningRuntimeActorGoalResult::Lowered {
                    goal_id: goal.goal_id,
                    plan,
                });
            return;
        }

        let planning_command_id =
            planning_command_id(&identified_request.request().request_id, &plan);
        let command_id = match resolve_planning_command_attempt(
            task_network,
            report,
            &goal.goal_id,
            &plan,
            &planning_command_id,
        ) {
            PlanningCommandAttemptResolution::Pending(command_id) => command_id,
            PlanningCommandAttemptResolution::Complete => return,
        };

        let task_ids = plan
            .mutations
            .mutations
            .iter()
            .map(|mutation| {
                let crate::task_network::mutation::Mutation::Inject(inject) = mutation;
                inject.task_node.task_instance_id.clone()
            })
            .collect();
        let task_network_materialization = match task_network.query().materialization(task_ids) {
            Ok(materialization) => {
                report.last_acknowledged_revision = report
                    .last_acknowledged_revision
                    .max(materialization.head.revision);
                materialization
            }
            Err(error) => {
                let retryable = task_network_authority_error_is_retryable(&error);
                let message = error.to_string();
                push_authority_issue(
                    report,
                    Some(goal.goal_id.clone()),
                    "task_network_materialization_query_failed",
                    error,
                );
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return;
            }
        };

        match plan_materialization(&task_network_materialization, &plan) {
            PlanMaterialization::Absent => {}
            PlanMaterialization::Exact => {
                let message = format!(
                    "planning composition '{}' is materialized without its exact durable command outcome",
                    plan.composition_id
                );
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_materialization_foreign".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable: false,
                    });
                return;
            }
            PlanMaterialization::Divergent(message) => {
                report.fatal_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_materialization_diverged".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable: false,
                    });
                return;
            }
        }

        match goals.planning_goal_record(&goal.goal_id) {
            Ok(Some(current))
                if current.updated_at_seq == goal_updated_at_seq
                    && current.goal == goal
                    && matches!(current.goal.lifecycle, GoalLifecycle::Active) => {}
            Ok(current) => {
                let message = match current {
                    Some(current) => format!(
                        "goal '{}' changed after planning selection: expected active sequence {}, observed sequence {} with lifecycle {:?}",
                        goal.goal_id,
                        goal_updated_at_seq,
                        current.updated_at_seq,
                        current.goal.lifecycle
                    ),
                    None => format!(
                        "goal '{}' disappeared after planning selection at sequence {}",
                        goal.goal_id, goal_updated_at_seq
                    ),
                };
                report.retryable_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_goal_fence_stale".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable: true,
                    });
                return;
            }
            Err(error) => {
                let retryable = error.retryable();
                let message = error.to_string();
                let issue = PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_goal_fence_query_failed".to_string(),
                    message: message.clone(),
                };
                if retryable {
                    report.retryable_errors.push(issue);
                } else {
                    report.fatal_errors.push(issue);
                }
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return;
            }
        }

        let base_revision = task_network_materialization.head.revision;
        let base_state_hash = task_network_materialization.head.state_hash;
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
        if let Err(message) = validate_planning_command_bounds(&command_request) {
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "planning_command_bounds_exceeded".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal.goal_id,
                    command_id: Some(command_id),
                    error: message,
                    retryable: false,
                });
            return;
        }
        let response = match task_network.commands().try_submit(command_request) {
            Ok(response) => response,
            Err(error) => {
                let retryable = task_network_authority_error_is_retryable(&error);
                let message = error.to_string();
                push_authority_issue(
                    report,
                    Some(goal.goal_id.clone()),
                    "task_network_command_failed",
                    error,
                );
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return;
            }
        };
        record_planning_command_response(report, goal.goal_id, plan, command_id, response);
    }

    fn persist_projection_failure(
        &self,
        store: &PlanningAttemptStore,
        projection_request: &PlanningWorldStateRequest,
        message: &str,
        report: &mut PlanningRuntimeActorReport,
    ) {
        let identity = PlanningProjectionFailureIdentityInputs::bind(
            projection_request.clone(),
            self.runtime.method_library.digest(),
            self.runtime.capability_catalog.digest(),
            PLANNING_IDENTITY_VERSION.to_string(),
        )
        .and_then(PlanningAttemptIdentity::bind_projection_failure);
        let identity = match identity {
            Ok(identity) => identity,
            Err(error) => {
                push_attempt_contract_failure(
                    report,
                    Some(projection_request.goal_id.clone()),
                    None,
                    "planning_projection_attempt_identity_failed",
                    error,
                );
                return;
            }
        };
        if let Err(error) = store.open_attempt(identity.clone()) {
            push_attempt_store_failure(
                report,
                Some(projection_request.goal_id.clone()),
                Some(identity.attempt_id().to_string()),
                "planning_projection_attempt_open_failed",
                error,
            );
            return;
        }
        let decision = PlanningAttemptDecisionAudit::new(
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            PlanningAttemptResultSummary::ProjectionFailed,
        );
        let decision = match decision {
            Ok(decision) => decision,
            Err(error) => {
                push_attempt_contract_failure(
                    report,
                    Some(projection_request.goal_id.clone()),
                    Some(identity.attempt_id().to_string()),
                    "planning_projection_attempt_audit_failed",
                    error,
                );
                return;
            }
        };
        persist_terminal_attempt(
            store,
            &identity,
            decision,
            PlanningAttemptDiagnosticDisposition::ProjectionFailed,
            "planning_projection_failed",
            message,
            report,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn process_identified_goal_durable<G>(
        &self,
        goals: &G,
        task_network: &TaskNetworkAuthorityPorts,
        request: &PlanningRuntimeActorRequest,
        goal: meld_lang::Goal,
        goal_updated_at_seq: u64,
        identified_request: IdentifiedPlanningRequest,
        attempt_store: &PlanningAttemptStore,
        pass: &mut PlanningActorPass,
    ) where
        G: GoalPlanningSelectionPort + ?Sized,
    {
        let report = &mut pass.report;
        let mut identity =
            match PlanningAttemptIdentity::bind(identified_request.identity().clone()) {
                Ok(identity) => identity,
                Err(error) => {
                    push_attempt_contract_failure(
                        report,
                        Some(goal.goal_id.clone()),
                        None,
                        "planning_attempt_identity_failed",
                        error,
                    );
                    return;
                }
            };
        let mut attempt_id = identity.attempt_id().to_string();
        if let Err(error) = attempt_store.open_attempt(identity.clone()) {
            push_attempt_store_failure(
                report,
                Some(goal.goal_id.clone()),
                Some(attempt_id),
                "planning_attempt_open_failed",
                error,
            );
            return;
        }

        let planning_result = match self.runtime.plan_goal(identified_request.request().clone()) {
            Ok(result) => result,
            Err(error) => {
                let message = format!("{error:?}");
                let audit = PlanningAttemptDecisionAudit::new(
                    None,
                    Vec::new(),
                    identified_request
                        .request()
                        .world_state_frame
                        .warnings
                        .clone(),
                    Vec::new(),
                    Vec::new(),
                    PlanningAttemptResultSummary::PlanningFailed,
                );
                if persist_terminal_audit_result(
                    attempt_store,
                    &identity,
                    audit,
                    PlanningAttemptDiagnosticDisposition::PlanningFailed,
                    "planning_input_failed",
                    &message,
                    report,
                ) {
                    report.fatal_errors.push(PlanningRuntimeActorIssue {
                        goal_id: Some(goal.goal_id.clone()),
                        code: "planning_input_failed".to_string(),
                        message,
                    });
                    report
                        .results
                        .push(PlanningRuntimeActorGoalResult::PlanningFailed {
                            goal_id: goal.goal_id,
                            error,
                        });
                }
                return;
            }
        };

        let PlanningResult::Composed(composition) = planning_result else {
            let audit = decision_audit_for_terminal_result(&planning_result);
            let terminal = terminal_spec_for_planning_result(&planning_result);
            if persist_terminal_audit_result(
                attempt_store,
                &identity,
                audit,
                terminal.disposition,
                terminal.code,
                terminal.message,
                report,
            ) {
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::Planned {
                        goal_id: goal.goal_id,
                        result: planning_result,
                    });
            }
            return;
        };

        let lower_request = CompositionLoweringRequest {
            request_id: lowering_request_id(&composition),
            network_id: request.network_id.clone(),
            idempotency_key: lowering_idempotency_key(&composition),
            composition: composition.clone(),
        };
        let composition_id = composition.composition_id.clone();
        let plan = match self.lowerer.lower(lower_request) {
            Ok(plan) => plan,
            Err(error) => {
                let message = error.to_string();
                let audit = decision_audit_for_composition(
                    &composition,
                    Vec::new(),
                    PlanningAttemptResultSummary::LoweringFailed {
                        composition_id: composition_id.clone(),
                    },
                );
                if persist_terminal_audit_result(
                    attempt_store,
                    &identity,
                    audit,
                    PlanningAttemptDiagnosticDisposition::LoweringFailed,
                    "composition_lowering_failed",
                    &message,
                    report,
                ) {
                    report.fatal_errors.push(PlanningRuntimeActorIssue {
                        goal_id: Some(goal.goal_id.clone()),
                        code: "composition_lowering_failed".to_string(),
                        message: message.clone(),
                    });
                    report
                        .results
                        .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                            goal_id: goal.goal_id,
                            composition_id,
                            error: message,
                        });
                }
                return;
            }
        };

        let retained_plan_bytes = match validate_planning_plan_bounds(&plan) {
            Ok(bytes) => bytes,
            Err(message) => {
                self.persist_durable_lowering_failure(
                    attempt_store,
                    &identity,
                    &composition,
                    &plan,
                    &goal.goal_id,
                    "planning_plan_bounds_exceeded",
                    message,
                    report,
                );
                return;
            }
        };
        if let Err(message) = pass.product_budget.reserve_plan(retained_plan_bytes) {
            self.persist_durable_lowering_failure(
                attempt_store,
                &identity,
                &composition,
                &plan,
                &goal.goal_id,
                "planning_tick_plan_budget_exhausted",
                message,
                report,
            );
            return;
        }

        if plan.mutations.mutations.is_empty() {
            let audit = decision_audit_for_composition(
                &composition,
                plan.diagnostics.clone(),
                PlanningAttemptResultSummary::NoMutation {
                    composition_id: composition_id.clone(),
                },
            );
            if persist_terminal_audit_result(
                attempt_store,
                &identity,
                audit,
                PlanningAttemptDiagnosticDisposition::NoMutation,
                "planning_lowering_no_mutation",
                "planning lowering produced no task network mutations",
                report,
            ) {
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::Lowered {
                        goal_id: goal.goal_id,
                        plan,
                    });
            }
            return;
        }

        let decision = decision_audit_for_composition(
            &composition,
            plan.diagnostics.clone(),
            PlanningAttemptResultSummary::Composed {
                composition_id: composition_id.clone(),
            },
        );
        let decision = match decision {
            Ok(decision) => decision,
            Err(error) => {
                push_attempt_contract_failure(
                    report,
                    Some(goal.goal_id.clone()),
                    Some(attempt_id.clone()),
                    "planning_attempt_audit_failed",
                    error,
                );
                return;
            }
        };
        if let Err(error) = attempt_store.record_decision(&attempt_id, decision.clone()) {
            push_attempt_store_failure(
                report,
                Some(goal.goal_id.clone()),
                Some(attempt_id.clone()),
                "planning_attempt_audit_persist_failed",
                error,
            );
            return;
        }

        let query = PlanningAttemptQuery::new(Arc::new(attempt_store.clone()));
        match query.prepared_command(&attempt_id) {
            Ok(Some(prepared)) => {
                if let Some(response) = authenticated_prepared_outcome(
                    attempt_store,
                    task_network,
                    &goal.goal_id,
                    &attempt_id,
                    &prepared,
                    report,
                ) {
                    let retryable_rejection = matches!(
                        &response,
                        command::Response::Rejected(rejection)
                            if planning_rejection_requires_new_attempt(rejection)
                    );
                    record_planning_command_response(
                        report,
                        goal.goal_id.clone(),
                        plan.clone(),
                        prepared.request.command_id.clone(),
                        response,
                    );
                    if !retryable_rejection {
                        return;
                    }
                    let parent_outcome = match query.command_outcome(&attempt_id) {
                        Ok(Some(outcome)) => outcome,
                        Ok(None) => {
                            push_attempt_contract_failure(
                                report,
                                Some(goal.goal_id.clone()),
                                Some(attempt_id.clone()),
                                "planning_rebase_parent_outcome_missing",
                                "completed retryable command has no durable planning outcome"
                                    .to_string(),
                            );
                            return;
                        }
                        Err(error) => {
                            push_attempt_store_failure(
                                report,
                                Some(goal.goal_id.clone()),
                                Some(attempt_id.clone()),
                                "planning_rebase_parent_outcome_query_failed",
                                error,
                            );
                            return;
                        }
                    };
                    let rebase_head = match task_network.query().head() {
                        Ok(head) => head,
                        Err(error) => {
                            let retryable = task_network_authority_error_is_retryable(&error);
                            let message = error.to_string();
                            push_authority_issue(
                                report,
                                Some(goal.goal_id.clone()),
                                "planning_rebase_head_query_failed",
                                error,
                            );
                            report
                                .results
                                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                                    goal_id: goal.goal_id,
                                    command_id: Some(prepared.request.command_id),
                                    error: message,
                                    retryable,
                                });
                            return;
                        }
                    };
                    let child = match PlanningAttemptIdentity::bind_rebase(
                        &identity,
                        &parent_outcome,
                        rebase_head.revision,
                        rebase_head.state_hash,
                    ) {
                        Ok(child) => child,
                        Err(error) => {
                            push_attempt_contract_failure(
                                report,
                                Some(goal.goal_id.clone()),
                                Some(attempt_id.clone()),
                                "planning_rebase_identity_failed",
                                error,
                            );
                            return;
                        }
                    };
                    if let Err(error) = attempt_store.open_rebase_attempt(child.clone()) {
                        push_attempt_store_failure(
                            report,
                            Some(goal.goal_id.clone()),
                            Some(child.attempt_id().to_string()),
                            "planning_rebase_attempt_open_failed",
                            error,
                        );
                        return;
                    }
                    if let Err(error) =
                        attempt_store.record_decision(child.attempt_id(), decision.clone())
                    {
                        push_attempt_store_failure(
                            report,
                            Some(goal.goal_id.clone()),
                            Some(child.attempt_id().to_string()),
                            "planning_rebase_audit_persist_failed",
                            error,
                        );
                        return;
                    }
                    identity = child;
                    attempt_id = identity.attempt_id().to_string();
                    match query.prepared_command(&attempt_id) {
                        Ok(Some(prepared)) => {
                            if let Some(response) = authenticated_prepared_outcome(
                                attempt_store,
                                task_network,
                                &goal.goal_id,
                                &attempt_id,
                                &prepared,
                                report,
                            ) {
                                record_planning_command_response(
                                    report,
                                    goal.goal_id,
                                    plan,
                                    prepared.request.command_id,
                                    response,
                                );
                            }
                            return;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            push_attempt_store_failure(
                                report,
                                Some(goal.goal_id.clone()),
                                Some(attempt_id.clone()),
                                "planning_rebase_prepared_command_query_failed",
                                error,
                            );
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
            Ok(None) => {}
            Err(error) => {
                push_attempt_store_failure(
                    report,
                    Some(goal.goal_id.clone()),
                    Some(attempt_id.clone()),
                    "planning_prepared_command_query_failed",
                    error,
                );
                return;
            }
        }

        let command_id = match planning_task_network_command_id(&identity, &composition_id) {
            Ok(command_id) => command_id,
            Err(error) => {
                push_attempt_contract_failure(
                    report,
                    Some(goal.goal_id.clone()),
                    Some(attempt_id.clone()),
                    "planning_command_identity_failed",
                    error,
                );
                return;
            }
        };
        let task_ids = plan
            .mutations
            .mutations
            .iter()
            .map(|mutation| {
                let crate::task_network::mutation::Mutation::Inject(inject) = mutation;
                inject.task_node.task_instance_id.clone()
            })
            .collect();
        let materialization = match task_network.query().materialization(task_ids) {
            Ok(materialization) => {
                report.last_acknowledged_revision = report
                    .last_acknowledged_revision
                    .max(materialization.head.revision);
                materialization
            }
            Err(error) => {
                let retryable = task_network_authority_error_is_retryable(&error);
                let message = error.to_string();
                push_authority_issue(
                    report,
                    Some(goal.goal_id.clone()),
                    "task_network_materialization_query_failed",
                    error,
                );
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return;
            }
        };
        if let Some((rebase_revision, rebase_state_hash)) = identity.rebase_base() {
            if materialization.head.revision != rebase_revision
                || materialization.head.state_hash != rebase_state_hash
            {
                let message = format!(
                    "task network head changed after planning rebase selection: expected revision {rebase_revision} and state {rebase_state_hash}, observed revision {} and state {}",
                    materialization.head.revision, materialization.head.state_hash
                );
                report.retryable_errors.push(PlanningRuntimeActorIssue {
                    goal_id: Some(goal.goal_id.clone()),
                    code: "planning_rebase_head_stale".to_string(),
                    message: message.clone(),
                });
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal.goal_id,
                        command_id: Some(command_id),
                        error: message,
                        retryable: true,
                    });
                return;
            }
        }
        match plan_materialization(&materialization, &plan) {
            PlanMaterialization::Absent => {}
            PlanMaterialization::Exact => {
                let message = format!(
                    "planning composition '{}' is materialized without its exact durable planning attempt outcome",
                    plan.composition_id
                );
                push_fatal_command_failure(
                    report,
                    goal.goal_id,
                    command_id,
                    "planning_materialization_foreign",
                    message,
                );
                return;
            }
            PlanMaterialization::Divergent(message) => {
                push_fatal_command_failure(
                    report,
                    goal.goal_id,
                    command_id,
                    "planning_materialization_diverged",
                    message,
                );
                return;
            }
        }

        if !revalidate_planning_goal(goals, &goal, goal_updated_at_seq, &command_id, report) {
            return;
        }

        let base_revision = materialization.head.revision;
        let base_state_hash = materialization.head.state_hash;
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
        if let Err(message) = validate_planning_command_bounds(&command_request) {
            push_fatal_command_failure(
                report,
                goal.goal_id,
                command_id,
                "planning_command_bounds_exceeded",
                message,
            );
            return;
        }
        if let Err(error) = attempt_store.prepare_command(&attempt_id, command_request) {
            push_attempt_store_failure(
                report,
                Some(goal.goal_id.clone()),
                Some(attempt_id.clone()),
                "planning_command_prepare_failed",
                error,
            );
            return;
        }
        let prepared = match query.prepared_command(&attempt_id) {
            Ok(Some(prepared)) => prepared,
            Ok(None) => {
                push_attempt_contract_failure(
                    report,
                    Some(goal.goal_id.clone()),
                    Some(attempt_id),
                    "planning_prepared_command_missing",
                    "durable command preparation completed without a recoverable request"
                        .to_string(),
                );
                return;
            }
            Err(error) => {
                push_attempt_store_failure(
                    report,
                    Some(goal.goal_id.clone()),
                    Some(attempt_id.clone()),
                    "planning_prepared_command_query_failed",
                    error,
                );
                return;
            }
        };
        if let Some(response) = authenticated_prepared_outcome(
            attempt_store,
            task_network,
            &goal.goal_id,
            &attempt_id,
            &prepared,
            report,
        ) {
            record_planning_command_response(
                report,
                goal.goal_id,
                plan,
                prepared.request.command_id,
                response,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn persist_durable_lowering_failure(
        &self,
        store: &PlanningAttemptStore,
        identity: &PlanningAttemptIdentity,
        composition: &ExecutionComposition,
        plan: &CompositionLoweringPlan,
        goal_id: &str,
        code: &str,
        message: String,
        report: &mut PlanningRuntimeActorReport,
    ) {
        let audit = decision_audit_for_composition(
            composition,
            plan.diagnostics.clone(),
            PlanningAttemptResultSummary::LoweringFailed {
                composition_id: composition.composition_id.clone(),
            },
        );
        if persist_terminal_audit_result(
            store,
            identity,
            audit,
            PlanningAttemptDiagnosticDisposition::LoweringFailed,
            code,
            &message,
            report,
        ) {
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal_id.to_string()),
                code: code.to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::LoweringFailed {
                    goal_id: goal_id.to_string(),
                    composition_id: composition.composition_id.clone(),
                    error: message,
                });
        }
    }
}

struct PlanningTerminalSpec {
    disposition: PlanningAttemptDiagnosticDisposition,
    code: &'static str,
    message: &'static str,
}

fn decision_audit_for_terminal_result(
    result: &PlanningResult,
) -> Result<PlanningAttemptDecisionAudit, String> {
    let (selected_method_id, candidate_reports, projection_warnings, method_diagnostics, summary) =
        match result {
            PlanningResult::Satisfied(satisfied) => (
                None,
                Vec::new(),
                satisfied.world_state_frame.warnings.clone(),
                satisfied.diagnostics.clone(),
                PlanningAttemptResultSummary::Satisfied,
            ),
            PlanningResult::NoApplicableMethod(no_method) => (
                None,
                no_method.candidates.clone(),
                no_method.world_state_frame.warnings.clone(),
                no_method.diagnostics.clone(),
                PlanningAttemptResultSummary::NoApplicableMethod,
            ),
            PlanningResult::Indeterminate(indeterminate) => (
                None,
                Vec::new(),
                indeterminate.world_state_frame.warnings.clone(),
                indeterminate.diagnostics.clone(),
                PlanningAttemptResultSummary::Indeterminate,
            ),
            PlanningResult::InvalidMethod(invalid) => (
                invalid.method_id.clone(),
                Vec::new(),
                Vec::new(),
                invalid.diagnostics.clone(),
                PlanningAttemptResultSummary::InvalidMethod,
            ),
            PlanningResult::Composed(_) => {
                return Err("composed planning result requires lowering audit context".to_string());
            }
        };
    PlanningAttemptDecisionAudit::new(
        selected_method_id,
        candidate_reports,
        projection_warnings,
        method_diagnostics,
        Vec::new(),
        summary,
    )
}

fn decision_audit_for_composition(
    composition: &ExecutionComposition,
    lowering_diagnostics: Vec<crate::planning::lowering::Diagnostic>,
    summary: PlanningAttemptResultSummary,
) -> Result<PlanningAttemptDecisionAudit, String> {
    PlanningAttemptDecisionAudit::new(
        Some(composition.method_id.clone()),
        Vec::new(),
        composition.world_state_frame.warnings.clone(),
        composition.diagnostics.clone(),
        lowering_diagnostics,
        summary,
    )
}

fn terminal_spec_for_planning_result(result: &PlanningResult) -> PlanningTerminalSpec {
    match result {
        PlanningResult::Satisfied(_) => PlanningTerminalSpec {
            disposition: PlanningAttemptDiagnosticDisposition::Satisfied,
            code: "planning_goal_satisfied",
            message: "projected world state already satisfies the active goal",
        },
        PlanningResult::NoApplicableMethod(_) => PlanningTerminalSpec {
            disposition: PlanningAttemptDiagnosticDisposition::NoApplicableMethod,
            code: "planning_no_applicable_method",
            message: "no verified planning method applies to the active goal",
        },
        PlanningResult::Indeterminate(_) => PlanningTerminalSpec {
            disposition: PlanningAttemptDiagnosticDisposition::Indeterminate,
            code: "planning_indeterminate",
            message: "projected world state cannot decide the active goal",
        },
        PlanningResult::InvalidMethod(_) => PlanningTerminalSpec {
            disposition: PlanningAttemptDiagnosticDisposition::InvalidMethod,
            code: "planning_invalid_method",
            message: "method verification prevented deterministic planning",
        },
        PlanningResult::Composed(_) => unreachable!("composed results are not terminal"),
    }
}

fn persist_terminal_audit_result(
    store: &PlanningAttemptStore,
    identity: &PlanningAttemptIdentity,
    audit: Result<PlanningAttemptDecisionAudit, String>,
    disposition: PlanningAttemptDiagnosticDisposition,
    code: &str,
    message: &str,
    report: &mut PlanningRuntimeActorReport,
) -> bool {
    let audit = match audit {
        Ok(audit) => audit,
        Err(error) => {
            push_attempt_contract_failure(
                report,
                Some(identity.goal_id().to_string()),
                Some(identity.attempt_id().to_string()),
                "planning_attempt_audit_failed",
                error,
            );
            return false;
        }
    };
    persist_terminal_attempt(store, identity, audit, disposition, code, message, report)
}

#[allow(clippy::too_many_arguments)]
fn persist_terminal_attempt(
    store: &PlanningAttemptStore,
    identity: &PlanningAttemptIdentity,
    decision: PlanningAttemptDecisionAudit,
    disposition: PlanningAttemptDiagnosticDisposition,
    code: &str,
    message: &str,
    report: &mut PlanningRuntimeActorReport,
) -> bool {
    if let Err(error) = store.record_decision(identity.attempt_id(), decision) {
        push_attempt_store_failure(
            report,
            Some(identity.goal_id().to_string()),
            Some(identity.attempt_id().to_string()),
            "planning_attempt_audit_persist_failed",
            error,
        );
        return false;
    }
    let diagnostic = match PlanningAttemptTerminalDiagnostic::new(
        disposition,
        code,
        bounded_diagnostic_message(message),
    ) {
        Ok(diagnostic) => diagnostic,
        Err(error) => {
            push_attempt_contract_failure(
                report,
                Some(identity.goal_id().to_string()),
                Some(identity.attempt_id().to_string()),
                "planning_terminal_diagnostic_invalid",
                error,
            );
            return false;
        }
    };
    if let Err(error) = store.record_terminal_diagnostic(identity.attempt_id(), diagnostic) {
        push_attempt_store_failure(
            report,
            Some(identity.goal_id().to_string()),
            Some(identity.attempt_id().to_string()),
            "planning_terminal_diagnostic_persist_failed",
            error,
        );
        return false;
    }
    true
}

fn bounded_diagnostic_message(message: &str) -> String {
    const MAX_BYTES: usize = 1_024;
    if message.len() <= MAX_BYTES {
        return message.to_string();
    }
    let mut end = MAX_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_string()
}

fn process_prepared_recovery(
    store: &PlanningAttemptStore,
    task_network: &TaskNetworkAuthorityPorts,
    request: &PlanningRuntimeActorRequest,
    recovery: PlanningAttemptRecovery,
    report: &mut PlanningRuntimeActorReport,
) {
    let goal_id = recovery.head.identity.goal_id().to_string();
    let attempt_id = recovery.head.identity.attempt_id().to_string();
    if recovery.prepared.request.network_id != request.network_id {
        push_fatal_command_failure(
            report,
            goal_id,
            recovery.prepared.request.command_id,
            "planning_recovery_network_mismatch",
            "prepared planning command targets a different task network".to_string(),
        );
        return;
    }
    let command_id = recovery.prepared.request.command_id.clone();
    if let Some(response) = authenticated_prepared_outcome(
        store,
        task_network,
        &goal_id,
        &attempt_id,
        &recovery.prepared,
        report,
    ) {
        record_recovered_command_response(report, goal_id, attempt_id, command_id, response);
    }
}

fn authenticated_prepared_outcome(
    store: &PlanningAttemptStore,
    task_network: &TaskNetworkAuthorityPorts,
    goal_id: &str,
    attempt_id: &str,
    prepared: &PlanningPreparedCommand,
    report: &mut PlanningRuntimeActorReport,
) -> Option<command::Response> {
    let command_id = prepared.request.command_id.clone();
    let mut receipt = match task_network.query().command_outcome(command_id.clone()) {
        Ok(receipt) => receipt,
        Err(error) => {
            let retryable = task_network_authority_error_is_retryable(&error);
            let message = error.to_string();
            push_authority_issue(
                report,
                Some(goal_id.to_string()),
                "task_network_command_outcome_query_failed",
                error,
            );
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal_id.to_string(),
                    command_id: Some(command_id),
                    error: message,
                    retryable,
                });
            return None;
        }
    };

    if receipt.is_none() {
        if let Err(error) = store.validate_owner_fence() {
            push_attempt_store_failure(
                report,
                Some(goal_id.to_string()),
                Some(attempt_id.to_string()),
                "planning_owner_fence_stale_before_submit",
                error,
            );
            return None;
        }
        let submit_error = task_network
            .commands()
            .try_submit(prepared.request.clone())
            .err();
        receipt = match task_network.query().command_outcome(command_id.clone()) {
            Ok(receipt) => receipt,
            Err(query_error) => {
                if let Some(submit_error) = submit_error {
                    push_authority_issue(
                        report,
                        Some(goal_id.to_string()),
                        "task_network_command_failed",
                        submit_error,
                    );
                }
                let retryable = task_network_authority_error_is_retryable(&query_error);
                let message = query_error.to_string();
                push_authority_issue(
                    report,
                    Some(goal_id.to_string()),
                    "task_network_command_outcome_query_failed",
                    query_error,
                );
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal_id.to_string(),
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return None;
            }
        };
        if receipt.is_none() {
            let message = submit_error.map_or_else(
                || {
                    "task network submission returned without an authenticated durable outcome"
                        .to_string()
                },
                |error| error.to_string(),
            );
            report.retryable_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal_id.to_string()),
                code: "task_network_command_outcome_pending".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal_id.to_string(),
                    command_id: Some(command_id),
                    error: message,
                    retryable: true,
                });
            return None;
        }
    }

    let receipt = receipt.expect("receipt presence checked");
    let response = receipt.response().clone();
    if let Err(error) = store.record_command_outcome(attempt_id, receipt) {
        push_attempt_store_failure(
            report,
            Some(goal_id.to_string()),
            Some(attempt_id.to_string()),
            "planning_command_outcome_persist_failed",
            error,
        );
        return None;
    }
    Some(response)
}

fn record_recovered_command_response(
    report: &mut PlanningRuntimeActorReport,
    goal_id: String,
    attempt_id: String,
    command_id: String,
    response: command::Response,
) {
    match &response {
        command::Response::Accepted { revision, .. }
        | command::Response::Duplicate { revision, .. } => {
            report.committed += 1;
            report.last_acknowledged_revision = report.last_acknowledged_revision.max(*revision);
        }
        command::Response::Rejected(rejection) => {
            let issue = PlanningRuntimeActorIssue {
                goal_id: Some(goal_id.clone()),
                code: "task_network_command_rejected".to_string(),
                message: format!("{rejection:?}"),
            };
            if task_network_rejection_is_retryable(rejection) {
                report.retryable_errors.push(issue);
            } else {
                report.fatal_errors.push(issue);
            }
        }
    }
    report
        .results
        .push(PlanningRuntimeActorGoalResult::Recovered {
            goal_id,
            attempt_id,
            command_id,
            response,
        });
}

fn revalidate_planning_goal<G>(
    goals: &G,
    goal: &meld_lang::Goal,
    goal_updated_at_seq: u64,
    command_id: &str,
    report: &mut PlanningRuntimeActorReport,
) -> bool
where
    G: GoalPlanningSelectionPort + ?Sized,
{
    match goals.planning_goal_record(&goal.goal_id) {
        Ok(Some(current))
            if current.updated_at_seq == goal_updated_at_seq
                && current.goal == *goal
                && matches!(current.goal.lifecycle, GoalLifecycle::Active) =>
        {
            true
        }
        Ok(current) => {
            let message = match current {
                Some(current) => format!(
                    "goal '{}' changed after planning selection: expected active sequence {}, observed sequence {} with lifecycle {:?}",
                    goal.goal_id,
                    goal_updated_at_seq,
                    current.updated_at_seq,
                    current.goal.lifecycle
                ),
                None => format!(
                    "goal '{}' disappeared after planning selection at sequence {}",
                    goal.goal_id, goal_updated_at_seq
                ),
            };
            report.retryable_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "planning_goal_fence_stale".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal.goal_id.clone(),
                    command_id: Some(command_id.to_string()),
                    error: message,
                    retryable: true,
                });
            false
        }
        Err(error) => {
            let retryable = error.retryable();
            let message = error.to_string();
            let issue = PlanningRuntimeActorIssue {
                goal_id: Some(goal.goal_id.clone()),
                code: "planning_goal_fence_query_failed".to_string(),
                message: message.clone(),
            };
            if retryable {
                report.retryable_errors.push(issue);
            } else {
                report.fatal_errors.push(issue);
            }
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal.goal_id.clone(),
                    command_id: Some(command_id.to_string()),
                    error: message,
                    retryable,
                });
            false
        }
    }
}

fn push_attempt_store_failure(
    report: &mut PlanningRuntimeActorReport,
    goal_id: Option<String>,
    attempt_id: Option<String>,
    code: &str,
    error: PlanningAttemptStorageError,
) {
    let retryable = error.is_retryable();
    let message = error.to_string();
    let issue = PlanningRuntimeActorIssue {
        goal_id: goal_id.clone(),
        code: code.to_string(),
        message: message.clone(),
    };
    if retryable {
        report.retryable_errors.push(issue);
    } else {
        report.fatal_errors.push(issue);
    }
    report
        .results
        .push(PlanningRuntimeActorGoalResult::AttemptFailed {
            goal_id,
            attempt_id,
            error: message,
            retryable,
        });
}

fn push_attempt_contract_failure(
    report: &mut PlanningRuntimeActorReport,
    goal_id: Option<String>,
    attempt_id: Option<String>,
    code: &str,
    message: String,
) {
    report.fatal_errors.push(PlanningRuntimeActorIssue {
        goal_id: goal_id.clone(),
        code: code.to_string(),
        message: message.clone(),
    });
    report
        .results
        .push(PlanningRuntimeActorGoalResult::AttemptFailed {
            goal_id,
            attempt_id,
            error: message,
            retryable: false,
        });
}

fn push_fatal_command_failure(
    report: &mut PlanningRuntimeActorReport,
    goal_id: String,
    command_id: String,
    code: &str,
    message: String,
) {
    report.fatal_errors.push(PlanningRuntimeActorIssue {
        goal_id: Some(goal_id.clone()),
        code: code.to_string(),
        message: message.clone(),
    });
    report
        .results
        .push(PlanningRuntimeActorGoalResult::CommandFailed {
            goal_id,
            command_id: Some(command_id),
            error: message,
            retryable: false,
        });
}

enum PlanningCommandAttemptResolution {
    Pending(String),
    Complete,
}

fn resolve_planning_command_attempt(
    task_network: &TaskNetworkAuthorityPorts,
    report: &mut PlanningRuntimeActorReport,
    goal_id: &str,
    plan: &CompositionLoweringPlan,
    planning_command_id: &str,
) -> PlanningCommandAttemptResolution {
    for attempt in 0..MAX_PLANNING_COMMAND_ATTEMPTS {
        let command_id = planning_command_attempt_id(planning_command_id, attempt);
        let receipt = match task_network.query().command_outcome(command_id.clone()) {
            Ok(receipt) => receipt,
            Err(error) => {
                let retryable = task_network_authority_error_is_retryable(&error);
                let message = error.to_string();
                push_authority_issue(
                    report,
                    Some(goal_id.to_string()),
                    "task_network_command_outcome_query_failed",
                    error,
                );
                report
                    .results
                    .push(PlanningRuntimeActorGoalResult::CommandFailed {
                        goal_id: goal_id.to_string(),
                        command_id: Some(command_id),
                        error: message,
                        retryable,
                    });
                return PlanningCommandAttemptResolution::Complete;
            }
        };
        let Some(receipt) = receipt else {
            return PlanningCommandAttemptResolution::Pending(command_id);
        };
        if let Err(message) = validate_planning_outcome_receipt(&receipt, &command_id, plan) {
            report.fatal_errors.push(PlanningRuntimeActorIssue {
                goal_id: Some(goal_id.to_string()),
                code: "planning_command_outcome_diverged".to_string(),
                message: message.clone(),
            });
            report
                .results
                .push(PlanningRuntimeActorGoalResult::CommandFailed {
                    goal_id: goal_id.to_string(),
                    command_id: Some(command_id),
                    error: message,
                    retryable: false,
                });
            return PlanningCommandAttemptResolution::Complete;
        }
        if matches!(
            receipt.response(),
            command::Response::Rejected(rejection)
                if planning_rejection_requires_new_attempt(rejection)
        ) {
            continue;
        }
        record_planning_command_response(
            report,
            goal_id.to_string(),
            plan.clone(),
            command_id,
            receipt.response().clone(),
        );
        return PlanningCommandAttemptResolution::Complete;
    }

    let message = format!(
        "planning command '{}' exhausted {MAX_PLANNING_COMMAND_ATTEMPTS} durable attempts",
        planning_command_id
    );
    report.fatal_errors.push(PlanningRuntimeActorIssue {
        goal_id: Some(goal_id.to_string()),
        code: "planning_command_attempts_exhausted".to_string(),
        message: message.clone(),
    });
    report
        .results
        .push(PlanningRuntimeActorGoalResult::CommandFailed {
            goal_id: goal_id.to_string(),
            command_id: None,
            error: message,
            retryable: false,
        });
    PlanningCommandAttemptResolution::Complete
}

fn planning_rejection_requires_new_attempt(rejection: &Rejection) -> bool {
    matches!(
        rejection,
        Rejection::StaleBase { .. }
            | Rejection::StateHashMismatch { .. }
            | Rejection::FailedPrecondition(_)
    )
}

fn record_planning_command_response(
    report: &mut PlanningRuntimeActorReport,
    goal_id: String,
    plan: CompositionLoweringPlan,
    command_id: String,
    response: command::Response,
) {
    match &response {
        command::Response::Accepted { revision, .. }
        | command::Response::Duplicate { revision, .. } => {
            report.committed += 1;
            report.last_acknowledged_revision = report.last_acknowledged_revision.max(*revision);
        }
        command::Response::Rejected(rejection) => {
            let issue = PlanningRuntimeActorIssue {
                goal_id: Some(goal_id.clone()),
                code: "task_network_command_rejected".to_string(),
                message: format!("{rejection:?}"),
            };
            if task_network_rejection_is_retryable(rejection) {
                report.retryable_errors.push(issue);
            } else {
                report.fatal_errors.push(issue);
            }
        }
    }
    report
        .results
        .push(PlanningRuntimeActorGoalResult::Submitted {
            goal_id,
            plan,
            command_id,
            response,
        });
}

fn planning_goal_store_error(error: GoalPlanningSelectionError) -> PlanningRuntimeActorError {
    let retryable = error.retryable();
    PlanningRuntimeActorError::GoalStore {
        message: error.to_string(),
        retryable,
    }
}

fn planning_authority_error(error: TaskNetworkAuthorityError) -> PlanningRuntimeActorError {
    let retryable = task_network_authority_error_is_retryable(&error);
    PlanningRuntimeActorError::TaskNetworkAuthority {
        message: error.to_string(),
        retryable,
    }
}

fn planning_attempt_authority_error(
    error: PlanningAttemptStorageError,
) -> PlanningRuntimeActorError {
    let retryable = error.is_retryable();
    PlanningRuntimeActorError::PlanningAttemptAuthority {
        message: error.to_string(),
        retryable,
    }
}

fn push_authority_issue(
    report: &mut PlanningRuntimeActorReport,
    goal_id: Option<String>,
    code: &str,
    error: TaskNetworkAuthorityError,
) {
    let retryable = task_network_authority_error_is_retryable(&error);
    let issue = PlanningRuntimeActorIssue {
        goal_id,
        code: code.to_string(),
        message: error.to_string(),
    };
    if retryable {
        report.retryable_errors.push(issue);
    } else {
        report.fatal_errors.push(issue);
    }
}

fn task_network_authority_error_is_retryable(error: &TaskNetworkAuthorityError) -> bool {
    match error {
        TaskNetworkAuthorityError::Full { .. }
        | TaskNetworkAuthorityError::Closing { .. }
        | TaskNetworkAuthorityError::Closed { .. }
        | TaskNetworkAuthorityError::StaleEpoch { .. }
        | TaskNetworkAuthorityError::Store(
            crate::task_network::store::TaskNetworkStoreError::Storage(_),
        ) => true,
        TaskNetworkAuthorityError::Poisoned { kind, .. } => kind.retryable(),
        _ => false,
    }
}

fn task_network_rejection_is_retryable(rejection: &Rejection) -> bool {
    matches!(
        rejection,
        Rejection::StaleBase { .. }
            | Rejection::StateHashMismatch { .. }
            | Rejection::FailedPrecondition(_)
            | Rejection::StaleClaim { .. }
            | Rejection::PublicationAlreadyMarked(_)
    )
}

#[derive(Debug, Default)]
struct PlanningTickProductBudget {
    projection_bytes: usize,
    retained_plan_bytes: usize,
}

struct PlanningActorPass {
    report: PlanningRuntimeActorReport,
    product_budget: PlanningTickProductBudget,
}

impl PlanningTickProductBudget {
    fn reserve_projection(&mut self, bytes: usize) -> Result<(), String> {
        let projection_bytes = self
            .projection_bytes
            .checked_add(bytes)
            .ok_or_else(|| "planning tick projection byte count overflowed".to_string())?;
        self.validate_total(projection_bytes, self.retained_plan_bytes)?;
        if projection_bytes > MAX_PLANNING_TICK_PROJECTION_BYTES {
            return Err(format!(
                "planning tick projection products require {projection_bytes} bytes, exceeding limit {MAX_PLANNING_TICK_PROJECTION_BYTES}"
            ));
        }
        self.projection_bytes = projection_bytes;
        Ok(())
    }

    fn reserve_plan(&mut self, bytes: usize) -> Result<(), String> {
        let retained_plan_bytes = self
            .retained_plan_bytes
            .checked_add(bytes)
            .ok_or_else(|| "planning tick retained plan byte count overflowed".to_string())?;
        self.validate_total(self.projection_bytes, retained_plan_bytes)?;
        if retained_plan_bytes > MAX_PLANNING_TICK_RETAINED_PLAN_BYTES {
            return Err(format!(
                "planning tick retained plans require {retained_plan_bytes} bytes, exceeding limit {MAX_PLANNING_TICK_RETAINED_PLAN_BYTES}"
            ));
        }
        self.retained_plan_bytes = retained_plan_bytes;
        Ok(())
    }

    fn validate_total(&self, projection_bytes: usize, plan_bytes: usize) -> Result<(), String> {
        let total = projection_bytes
            .checked_add(plan_bytes)
            .ok_or_else(|| "planning tick product byte count overflowed".to_string())?;
        if total > MAX_PLANNING_TICK_PRODUCT_BYTES {
            return Err(format!(
                "planning tick products require {total} bytes, exceeding aggregate limit {MAX_PLANNING_TICK_PRODUCT_BYTES}"
            ));
        }
        Ok(())
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
    if request.branch_id.len() > MAX_PLANNING_TEXT_BYTES
        || request.perspective.perspective_kind.len() > MAX_PLANNING_TEXT_BYTES
        || request.perspective.perspective_id.len() > MAX_PLANNING_TEXT_BYTES
    {
        return Err(PlanningRuntimeActorError::InvalidRequest(format!(
            "projection scope identity must not exceed {MAX_PLANNING_TEXT_BYTES} bytes"
        )));
    }
    if request.requested_dimensions.len() > MAX_PLANNING_PROJECTION_ITEMS
        || request.required_preconditions.len() > MAX_PLANNING_PROJECTION_ITEMS
    {
        return Err(PlanningRuntimeActorError::InvalidRequest(format!(
            "projection request collections must not exceed {MAX_PLANNING_PROJECTION_ITEMS} items"
        )));
    }
    if request
        .requested_dimensions
        .iter()
        .any(|dimension| dimension.trim().is_empty() || dimension.len() > MAX_PLANNING_TEXT_BYTES)
    {
        return Err(PlanningRuntimeActorError::InvalidRequest(format!(
            "projection dimensions must be non-empty and at most {MAX_PLANNING_TEXT_BYTES} bytes"
        )));
    }
    let encoded_preconditions = serde_json::to_vec(&request.required_preconditions)
        .map_err(|error| PlanningRuntimeActorError::InvalidRequest(error.to_string()))?;
    if encoded_preconditions.len() > MAX_PLANNING_PROJECTION_BYTES {
        return Err(PlanningRuntimeActorError::InvalidRequest(format!(
            "projection preconditions must not exceed {MAX_PLANNING_PROJECTION_BYTES} encoded bytes"
        )));
    }
    if request.limit == Some(0) {
        return Err(PlanningRuntimeActorError::InvalidRequest(
            "limit must be greater than zero".to_string(),
        ));
    }
    if request
        .limit
        .is_some_and(|limit| limit > MAX_PLANNING_SELECTION_LIMIT)
    {
        return Err(PlanningRuntimeActorError::InvalidRequest(format!(
            "limit must not exceed {MAX_PLANNING_SELECTION_LIMIT}"
        )));
    }
    Ok(())
}

fn validate_projection_bounds(projection: &PlanningWorldStateProjection) -> Result<usize, String> {
    if projection.world_state.propositions().len() > MAX_PLANNING_PROJECTION_ITEMS {
        return Err(format!(
            "projected world state contains {} propositions, exceeding limit {MAX_PLANNING_PROJECTION_ITEMS}",
            projection.world_state.propositions().len()
        ));
    }
    if projection.frame.source_refs.len() > MAX_PLANNING_PROJECTION_ITEMS
        || projection.frame.warnings.len() > MAX_PLANNING_PROJECTION_ITEMS
    {
        return Err(format!(
            "projection provenance collections must not exceed {MAX_PLANNING_PROJECTION_ITEMS} items"
        ));
    }
    let frame_text = [
        &projection.frame.frame_id,
        &projection.frame.request_id,
        &projection.frame.source_request_hash,
        &projection.frame.projection_version,
        &projection.frame.projection_hash,
        &projection.frame.world_state_hash,
        &projection.frame.perspective_kind,
        &projection.frame.perspective_id,
        &projection.frame.branch_id,
    ];
    if frame_text
        .into_iter()
        .chain(projection.frame.source_refs.iter())
        .chain(projection.frame.warnings.iter())
        .any(|value| value.trim().is_empty() || value.len() > MAX_PLANNING_TEXT_BYTES)
    {
        return Err(format!(
            "projection provenance text must be non-empty and at most {MAX_PLANNING_TEXT_BYTES} bytes"
        ));
    }
    let world_state_bytes = serde_json::to_vec(&projection.world_state)
        .map_err(|error| format!("projected world state encoding failed: {error}"))?;
    let frame_bytes = serde_json::to_vec(&projection.frame)
        .map_err(|error| format!("projection frame encoding failed: {error}"))?;
    if world_state_bytes.len() > MAX_PLANNING_PROJECTION_BYTES
        || frame_bytes.len() > MAX_PLANNING_PROJECTION_BYTES
    {
        return Err(format!(
            "projection products must not exceed {MAX_PLANNING_PROJECTION_BYTES} encoded bytes"
        ));
    }
    world_state_bytes
        .len()
        .checked_add(frame_bytes.len())
        .ok_or_else(|| "projection product byte count overflowed".to_string())
}

fn projection_request_for_goal(
    request: &PlanningRuntimeActorRequest,
    goal: &meld_lang::Goal,
    goal_updated_at_seq: u64,
) -> Result<PlanningWorldStateRequest, String> {
    PlanningWorldStateRequest::for_goal(
        goal,
        goal_updated_at_seq,
        request.perspective.clone(),
        request.branch_id.clone(),
        request.requested_dimensions.clone(),
        request.required_preconditions.clone(),
    )
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

enum PlanMaterialization {
    Absent,
    Exact,
    Divergent(String),
}

fn plan_materialization(
    materialization: &TaskNetworkMaterialization,
    plan: &CompositionLoweringPlan,
) -> PlanMaterialization {
    let mut present = 0;
    for mutation in &plan.mutations.mutations {
        let crate::task_network::mutation::Mutation::Inject(inject) = mutation;
        let task_id = &inject.task_node.task_instance_id;
        let Some(actual) = materialization.tasks.get(task_id) else {
            continue;
        };
        present += 1;
        if actual != &inject.task_node {
            return PlanMaterialization::Divergent(format!(
                "task '{}' is materialized with a divergent task node",
                task_id
            ));
        }
        let expected_edges = inject
            .incoming_edges
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let actual_edges = materialization
            .incoming_edges
            .get(task_id)
            .into_iter()
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>();
        if actual_edges != expected_edges {
            return PlanMaterialization::Divergent(format!(
                "task '{}' is materialized with divergent incoming edges",
                task_id
            ));
        }
    }
    if present == 0 {
        PlanMaterialization::Absent
    } else if present == plan.mutations.mutations.len() {
        PlanMaterialization::Exact
    } else {
        PlanMaterialization::Divergent(format!(
            "planning composition '{}' is only partially materialized",
            plan.composition_id
        ))
    }
}

fn planning_command_id(planning_request_id: &str, plan: &CompositionLoweringPlan) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        planning_request_id: &'a str,
        network_id: &'a str,
        composition_id: &'a str,
        mutation_set_id: &'a str,
    }

    stable_runtime_id(
        "execution-planning-task-network-command",
        &Identity {
            planning_request_id,
            network_id: &plan.network_id,
            composition_id: &plan.composition_id,
            mutation_set_id: &plan.mutations.set_id,
        },
    )
}

fn planning_command_attempt_id(planning_command_id: &str, attempt: usize) -> String {
    if attempt == 0 {
        return planning_command_id.to_string();
    }
    #[derive(Serialize)]
    struct Identity<'a> {
        planning_command_id: &'a str,
        attempt: usize,
    }

    // The ordinal makes the immutable outcome chain discoverable after reopen.
    // Each stored request then binds that attempt to its exact authority head.
    stable_runtime_id(
        "execution-planning-task-network-attempt",
        &Identity {
            planning_command_id,
            attempt,
        },
    )
}

fn validate_planning_plan_bounds(plan: &CompositionLoweringPlan) -> Result<usize, String> {
    if plan.mutations.mutations.len() > MAX_PLANNING_TASKS_PER_GOAL {
        return Err(format!(
            "planning composition '{}' contains {} task mutations, exceeding limit {MAX_PLANNING_TASKS_PER_GOAL}",
            plan.composition_id,
            plan.mutations.mutations.len()
        ));
    }
    for mutation in &plan.mutations.mutations {
        let crate::task_network::mutation::Mutation::Inject(inject) = mutation;
        if inject.task_node.task_instance_id.trim().is_empty()
            || inject.task_node.task_instance_id.len() > MAX_PLANNING_TASK_ID_BYTES
        {
            return Err(format!(
                "planning task identity has invalid byte length {}",
                inject.task_node.task_instance_id.len()
            ));
        }
        if inject.incoming_edges.len() > MAX_PLANNING_INCOMING_EDGES_PER_TASK {
            return Err(format!(
                "planning task '{}' contains {} incoming edges, exceeding limit {MAX_PLANNING_INCOMING_EDGES_PER_TASK}",
                inject.task_node.task_instance_id,
                inject.incoming_edges.len()
            ));
        }
        if let Some(edge) = inject.incoming_edges.iter().find(|edge| {
            edge.from.trim().is_empty()
                || edge.to.trim().is_empty()
                || edge.from.len() > MAX_PLANNING_TASK_ID_BYTES
                || edge.to.len() > MAX_PLANNING_TASK_ID_BYTES
        }) {
            return Err(format!(
                "planning dependency edge from '{}' to '{}' exceeds identity bounds",
                edge.from, edge.to
            ));
        }
    }
    let encoded_mutations = serde_json::to_vec(&plan.mutations)
        .map_err(|error| format!("planning mutation set encoding failed: {error}"))?;
    if encoded_mutations.len() > MAX_PLANNING_COMMAND_BYTES {
        return Err(format!(
            "planning composition '{}' encodes to {} command bytes, exceeding limit {MAX_PLANNING_COMMAND_BYTES}",
            plan.composition_id,
            encoded_mutations.len()
        ));
    }
    let encoded_plan = serde_json::to_vec(plan)
        .map_err(|error| format!("planning retained plan encoding failed: {error}"))?;
    Ok(encoded_plan.len())
}

fn validate_planning_command_bounds(request: &command::Request) -> Result<usize, String> {
    #[derive(Serialize)]
    struct DurableRequestEnvelope<'a> {
        request_hash: String,
        request: &'a command::Request,
    }

    let encoded = serde_json::to_vec(&DurableRequestEnvelope {
        request_hash: command::request_hash(request),
        request,
    })
    .map_err(|error| format!("planning command request encoding failed: {error}"))?;
    if encoded.len() > MAX_PLANNING_COMMAND_BYTES {
        return Err(format!(
            "planning command '{}' durable request encodes to {} bytes, exceeding complete request limit {MAX_PLANNING_COMMAND_BYTES}",
            request.command_id,
            encoded.len()
        ));
    }
    Ok(encoded.len())
}

fn validate_planning_outcome_receipt(
    receipt: &command::OutcomeReceipt,
    command_id: &str,
    plan: &CompositionLoweringPlan,
) -> Result<(), String> {
    if receipt.command_id() != command_id {
        return Err("durable planning outcome command identity mismatch".to_string());
    }
    let durable = receipt.request();
    if durable.command_id != command_id
        || durable.network_id != plan.network_id
        || receipt.request_hash() != command::request_hash(durable)
    {
        return Err("durable planning outcome request identity mismatch".to_string());
    }
    let TaskNetworkCommand::ApplyMutationSet(durable_mutations) = &durable.command else {
        return Err("durable planning outcome contains a foreign command kind".to_string());
    };
    if durable_mutations != &plan.mutations {
        return Err("durable planning outcome contains a foreign mutation set".to_string());
    }
    let expected_preconditions = [
        ReadPrecondition::RevisionIs(durable.base_revision),
        ReadPrecondition::StateHashIs(durable.base_state_hash.clone()),
    ];
    if durable.base_state_hash.trim().is_empty()
        || durable.read_preconditions.as_slice() != expected_preconditions
    {
        return Err("durable planning outcome continuation fence mismatch".to_string());
    }
    match receipt.response() {
        command::Response::Accepted {
            revision,
            state_hash,
        } => {
            if durable.base_revision.checked_add(1) != Some(*revision)
                || state_hash.trim().is_empty()
            {
                return Err("durable planning outcome acceptance mismatch".to_string());
            }
        }
        command::Response::Duplicate { .. } => {
            return Err("durable planning outcome cannot contain replay-form response".to_string());
        }
        command::Response::Rejected(_) => {}
    }
    Ok(())
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

#[cfg(test)]
mod actor_classification_tests {
    use super::*;
    use crate::task_network::mutation::Set;
    use crate::task_network::store::TaskNetworkStoreFactory;
    use crate::task_network::{TaskNetworkAuthority, TaskNetworkAuthorityPoisonKind};

    fn empty_command(
        state: &crate::task_network::NetworkState,
        command_id: &str,
    ) -> command::Request {
        command::Request {
            command_id: command_id.to_string(),
            network_id: state.network_id.clone(),
            base_revision: state.revision,
            base_state_hash: state.state_hash.clone(),
            read_preconditions: vec![
                ReadPrecondition::RevisionIs(state.revision),
                ReadPrecondition::StateHashIs(state.state_hash.clone()),
            ],
            command: TaskNetworkCommand::ApplyMutationSet(Set::empty(
                state.network_id.clone(),
                format!("composition-{command_id}"),
                format!("dedupe-{command_id}"),
            )),
        }
    }

    #[test]
    fn stale_authority_views_are_retryable_but_command_identity_conflicts_are_fatal() {
        assert!(task_network_rejection_is_retryable(&Rejection::StaleBase {
            expected: 4,
            actual: 5,
        }));
        assert!(task_network_rejection_is_retryable(
            &Rejection::StateHashMismatch {
                expected: "old".to_string(),
                actual: "new".to_string(),
            }
        ));
        assert!(!task_network_rejection_is_retryable(
            &Rejection::DuplicateCommand("planning-command".to_string())
        ));
        assert!(!task_network_rejection_is_retryable(
            &Rejection::InvalidGraph("invalid lowering".to_string())
        ));
    }

    #[test]
    fn paired_authority_turns_a_stale_query_race_into_a_retryable_rejection() {
        let temp = tempfile::tempdir().unwrap();
        let factory = TaskNetworkStoreFactory::new(temp.path());
        let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 8).unwrap();
        let ports = authority.ports();
        let stale = ports.query().state().unwrap();

        assert!(matches!(
            ports
                .commands()
                .try_submit(empty_command(&stale, "winner"))
                .unwrap(),
            command::Response::Accepted { revision: 1, .. }
        ));
        let response = ports
            .commands()
            .try_submit(empty_command(&stale, "stale-planner"))
            .unwrap();

        let command::Response::Rejected(rejection) = response else {
            panic!("stale planning command must be rejected");
        };
        assert!(matches!(rejection, Rejection::StaleBase { actual: 1, .. }));
        assert!(task_network_rejection_is_retryable(&rejection));
        authority.shutdown().unwrap();
    }

    #[test]
    fn poison_retryability_distinguishes_storage_from_corrupt_state() {
        let storage = TaskNetworkAuthorityError::Poisoned {
            network_id: "network-docs".to_string(),
            kind: TaskNetworkAuthorityPoisonKind::Storage,
            reason: "temporary storage outage".to_string(),
        };
        let corrupt = TaskNetworkAuthorityError::Poisoned {
            network_id: "network-docs".to_string(),
            kind: TaskNetworkAuthorityPoisonKind::CorruptState,
            reason: "malformed durable epoch".to_string(),
        };

        assert!(task_network_authority_error_is_retryable(&storage));
        assert!(!task_network_authority_error_is_retryable(&corrupt));
    }

    #[test]
    fn goal_read_retryability_distinguishes_io_from_corrupt_records() {
        let storage =
            GoalPlanningSelectionError::TransientStorage("temporary read failure".to_string());
        let corrupt = GoalPlanningSelectionError::CorruptState("invalid goal record".to_string());

        assert!(planning_goal_store_error(storage).retryable());
        assert!(!planning_goal_store_error(corrupt).retryable());
    }

    #[test]
    fn complete_command_request_cap_accepts_exact_boundary_and_rejects_next_byte() {
        let state = crate::task_network::NetworkState::empty("network-docs");
        let mut request = empty_command(&state, "bounded-command");
        request.base_state_hash.clear();
        let baseline = validate_planning_command_bounds(&request).unwrap();
        request.base_state_hash = "x".repeat(MAX_PLANNING_COMMAND_BYTES - baseline);

        assert_eq!(
            validate_planning_command_bounds(&request).unwrap(),
            MAX_PLANNING_COMMAND_BYTES
        );

        request.base_state_hash.push('x');
        assert!(validate_planning_command_bounds(&request).is_err());
    }

    #[test]
    fn retained_plan_budget_counts_the_complete_serialized_plan() {
        let plan = CompositionLoweringPlan {
            request_id: "planning-request".to_string(),
            network_id: "network-docs".to_string(),
            composition_id: "composition-docs".to_string(),
            goal_id: "goal-docs".to_string(),
            method_id: "method-docs".to_string(),
            mutations: crate::task_network::mutation::Set::empty(
                "network-docs",
                "composition-docs",
                "planning-once",
            ),
            diagnostics: Vec::new(),
        };

        let measured = validate_planning_plan_bounds(&plan).unwrap();

        assert_eq!(measured, serde_json::to_vec(&plan).unwrap().len());
        assert!(measured > serde_json::to_vec(&plan.mutations).unwrap().len());
    }

    #[test]
    fn tick_product_budget_enforces_projection_plan_and_aggregate_boundaries() {
        let mut budget = PlanningTickProductBudget::default();
        budget
            .reserve_projection(MAX_PLANNING_TICK_PROJECTION_BYTES)
            .unwrap();
        budget
            .reserve_plan(MAX_PLANNING_TICK_PRODUCT_BYTES - MAX_PLANNING_TICK_PROJECTION_BYTES)
            .unwrap();
        assert!(budget.reserve_plan(1).is_err());

        let mut projection = PlanningTickProductBudget::default();
        projection
            .reserve_projection(MAX_PLANNING_TICK_PROJECTION_BYTES)
            .unwrap();
        assert!(projection.reserve_projection(1).is_err());

        let mut plans = PlanningTickProductBudget::default();
        plans
            .reserve_plan(MAX_PLANNING_TICK_RETAINED_PLAN_BYTES)
            .unwrap();
        assert!(plans.reserve_plan(1).is_err());
    }
}
