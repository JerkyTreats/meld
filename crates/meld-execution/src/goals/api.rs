//! Producer-neutral goal acceptance at the execution boundary.
//!
//! Producers validate their own domain-specific command objects before mapping
//! them into these request types. Execution validates shared invariants, owns
//! lifecycle state, and persists the resulting goal command outcome.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, GoalCommandMetadata, GoalCommandOutcome, ModifyGoalCommand, RemoveGoalCommand,
    ResumeGoalCommand, SatisfyGoalCommand, SuspendGoalCommand,
};
use crate::goals::persistent_store::PersistentGoalSetStore;
use crate::goals::store::GoalSetStore;
use meld_lang::GoalLifecycle;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Request to accept one producer-authored goal into execution storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalAcceptanceRequest {
    /// Idempotency metadata for exact replay and optional producer dedupe.
    pub metadata: GoalCommandMetadata,
    /// Ground goal whose lifecycle must match `lifecycle_policy`.
    pub goal: meld_lang::Goal,
    /// Contract for how execution should interpret the incoming lifecycle.
    pub lifecycle_policy: GoalAcceptanceLifecycle,
}

/// Lifecycle contract enforced before an accepted goal reaches storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalAcceptanceLifecycle {
    /// Require producer intent to be proposed and store the execution copy as active.
    RequireProposedThenActivate,
    /// Require the producer to supply an already active execution-facing goal.
    RequireActive,
}

/// Errors returned by the execution goal set facade.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GoalSetApiError {
    /// The request violated shared execution boundary invariants.
    #[error("invalid goal acceptance request: {0}")]
    InvalidCommand(String),
    /// Storage rejected a validated command.
    #[error("goal acceptance failed: {0}")]
    Store(String),
}

/// Minimal mutating goal-store surface required by `GoalSetApi`.
///
/// The facade uses this trait to apply the same boundary validation to both
/// in-memory tests and durable execution state.
pub trait GoalSetCommandStore {
    /// Persist or replay an add-goal command.
    fn add_goal_command(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;

    /// Persist or replay a complete goal replacement.
    fn modify_goal_command(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;

    /// Persist or replay an abandonment transition.
    fn remove_goal_command(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;

    /// Persist or replay a satisfaction transition.
    fn satisfy_goal_command(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;

    /// Persist or replay a suspension transition.
    fn suspend_goal_command(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;

    /// Persist or replay a transition back to active.
    fn resume_goal_command(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError>;
}

impl GoalSetCommandStore for GoalSetStore {
    fn add_goal_command(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.add_goal(command)
    }

    fn modify_goal_command(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.modify_goal(command)
    }

    fn remove_goal_command(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.remove_goal(command)
    }

    fn satisfy_goal_command(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.satisfy_goal(command)
    }

    fn suspend_goal_command(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.suspend_goal(command)
    }

    fn resume_goal_command(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.resume_goal(command)
    }
}

impl GoalSetCommandStore for PersistentGoalSetStore {
    fn add_goal_command(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.add_goal(command)
    }

    fn modify_goal_command(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.modify_goal(command)
    }

    fn remove_goal_command(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.remove_goal(command)
    }

    fn satisfy_goal_command(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.satisfy_goal(command)
    }

    fn suspend_goal_command(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.suspend_goal(command)
    }

    fn resume_goal_command(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.resume_goal(command)
    }
}

/// Execution-owned facade for accepted goals and lifecycle commands.
pub struct GoalSetApi<'a, S> {
    store: &'a mut S,
}

impl<'a, S> GoalSetApi<'a, S>
where
    S: GoalSetCommandStore,
{
    /// Bind the facade to one mutable command store.
    pub fn new(store: &'a mut S) -> Self {
        Self { store }
    }

    /// Validate and store a producer-neutral goal acceptance request.
    pub fn accept_goal(
        &mut self,
        request: GoalAcceptanceRequest,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        let command = build_acceptance_add_goal_command(request)?;
        self.add_goal(command)
    }

    /// Apply an execution-native add-goal command through the same error surface.
    pub fn add_goal(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .add_goal_command(command)
            .map_err(map_store_error)
    }

    /// Apply an execution-native goal replacement through the facade.
    pub fn modify_goal(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .modify_goal_command(command)
            .map_err(map_store_error)
    }

    /// Apply an execution-native abandonment command through the facade.
    pub fn remove_goal(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .remove_goal_command(command)
            .map_err(map_store_error)
    }

    /// Apply an execution-native satisfaction command through the facade.
    pub fn satisfy_goal(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .satisfy_goal_command(command)
            .map_err(map_store_error)
    }

    /// Apply an execution-native suspension command through the facade.
    pub fn suspend_goal(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .suspend_goal_command(command)
            .map_err(map_store_error)
    }

    /// Apply an execution-native resume command through the facade.
    pub fn resume_goal(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.store
            .resume_goal_command(command)
            .map_err(map_store_error)
    }
}

fn build_acceptance_add_goal_command(
    request: GoalAcceptanceRequest,
) -> Result<AddGoalCommand, GoalSetApiError> {
    let GoalAcceptanceRequest {
        metadata,
        mut goal,
        lifecycle_policy,
    } = request;
    require_non_empty("goal command id", &metadata.command_id)?;
    if let Some(source_identity) = &metadata.source_identity {
        require_non_empty("goal source identity", source_identity)?;
    }
    require_non_empty("goal id", &goal.goal_id)?;
    require_non_empty("agent id", &goal.agent_id)?;
    if let Some(variable) = goal.target.grounding_issue() {
        return Err(GoalSetApiError::InvalidCommand(format!(
            "goal target must be ground: {variable}"
        )));
    }

    match lifecycle_policy {
        GoalAcceptanceLifecycle::RequireProposedThenActivate => {
            if !matches!(goal.lifecycle, GoalLifecycle::Proposed) {
                return Err(GoalSetApiError::InvalidCommand(
                    "goal lifecycle must be proposed".to_string(),
                ));
            }
            goal.lifecycle = GoalLifecycle::Active;
        }
        GoalAcceptanceLifecycle::RequireActive => {
            if !matches!(goal.lifecycle, GoalLifecycle::Active) {
                return Err(GoalSetApiError::InvalidCommand(
                    "goal lifecycle must be active".to_string(),
                ));
            }
        }
    }

    Ok(AddGoalCommand { metadata, goal })
}

fn require_non_empty(label: &str, value: &str) -> Result<(), GoalSetApiError> {
    if value.trim().is_empty() {
        return Err(GoalSetApiError::InvalidCommand(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

fn map_store_error(error: ExecutionInvariantError) -> GoalSetApiError {
    GoalSetApiError::Store(error.to_string())
}
