//! Producer-neutral goal acceptance at the execution boundary.
//!
//! Producers validate their own domain-specific command objects before mapping
//! them into these request types. Execution validates shared invariants, owns
//! lifecycle state, and persists the resulting goal command outcome.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, GoalCommandCommitReceipt, GoalCommandKind, GoalCommandMetadata,
    GoalCommandOutcome, GoalCommandRequestContract, GoalCommandRequestIdentity, ModifyGoalCommand,
    RemoveGoalCommand, ResumeGoalCommand, SatisfyGoalCommand, SuspendGoalCommand,
};
use crate::goals::persistent_store::PersistentGoalSetStore;
use crate::goals::store::{request_identity, GoalSetStore};
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

impl GoalCommandRequestContract for GoalAcceptanceRequest {
    fn metadata(&self) -> &GoalCommandMetadata {
        &self.metadata
    }

    fn command_kind(&self) -> GoalCommandKind {
        GoalCommandKind::Add
    }
}

mod private {
    use super::*;

    pub trait GoalSetCommandStore {
        /// Atomically persist or replay an add-goal command and its request identity.
        fn commit_add_goal_command(
            &mut self,
            command: AddGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Atomically persist or replay a complete goal replacement.
        fn commit_modify_goal_command(
            &mut self,
            command: ModifyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Atomically persist or replay an abandonment transition.
        fn commit_remove_goal_command(
            &mut self,
            command: RemoveGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Atomically persist or replay a satisfaction transition.
        fn commit_satisfy_goal_command(
            &mut self,
            command: SatisfyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Atomically persist or replay a suspension transition.
        fn commit_suspend_goal_command(
            &mut self,
            command: SuspendGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Atomically persist or replay a transition back to active.
        fn commit_resume_goal_command(
            &mut self,
            command: ResumeGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>;

        /// Flush every goal command tree before a durable commit is reported.
        fn flush_goal_commands(&mut self) -> Result<(), ExecutionInvariantError>;
    }

    impl GoalSetCommandStore for GoalSetStore {
        fn commit_add_goal_command(
            &mut self,
            command: AddGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_add_goal_with_identity(command, identity)
        }

        fn commit_modify_goal_command(
            &mut self,
            command: ModifyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_modify_goal_with_identity(command, identity)
        }

        fn commit_remove_goal_command(
            &mut self,
            command: RemoveGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_remove_goal_with_identity(command, identity)
        }

        fn commit_satisfy_goal_command(
            &mut self,
            command: SatisfyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_satisfy_goal_with_identity(command, identity)
        }

        fn commit_suspend_goal_command(
            &mut self,
            command: SuspendGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_suspend_goal_with_identity(command, identity)
        }

        fn commit_resume_goal_command(
            &mut self,
            command: ResumeGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_resume_goal_with_identity(command, identity)
        }

        fn flush_goal_commands(&mut self) -> Result<(), ExecutionInvariantError> {
            Ok(())
        }
    }

    impl GoalSetCommandStore for PersistentGoalSetStore {
        fn commit_add_goal_command(
            &mut self,
            command: AddGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_add_goal_with_identity(command, identity)
        }

        fn commit_modify_goal_command(
            &mut self,
            command: ModifyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_modify_goal_with_identity(command, identity)
        }

        fn commit_remove_goal_command(
            &mut self,
            command: RemoveGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_remove_goal_with_identity(command, identity)
        }

        fn commit_satisfy_goal_command(
            &mut self,
            command: SatisfyGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_satisfy_goal_with_identity(command, identity)
        }

        fn commit_suspend_goal_command(
            &mut self,
            command: SuspendGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_suspend_goal_with_identity(command, identity)
        }

        fn commit_resume_goal_command(
            &mut self,
            command: ResumeGoalCommand,
            identity: GoalCommandRequestIdentity,
        ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError>
        {
            self.commit_resume_goal_with_identity(command, identity)
        }

        fn flush_goal_commands(&mut self) -> Result<(), ExecutionInvariantError> {
            self.flush()
        }
    }
}

/// Sealed store capability accepted by `GoalSetApi`.
///
/// Command identity and pre-flush receipt methods remain private to the
/// facade, so callers can submit only complete canonical requests.
///
/// ```compile_fail
/// use meld_execution::goals::api::GoalSetApiStore;
/// use meld_execution::goals::{AddGoalCommand, GoalCommandRequestIdentity, GoalSetStore};
/// let mut store = GoalSetStore::new();
/// let command: AddGoalCommand = todo!();
/// let forged_identity: GoalCommandRequestIdentity = todo!();
/// store.commit_add_goal_command(command, forged_identity);
/// ```
pub trait GoalSetApiStore: private::GoalSetCommandStore {}

impl GoalSetApiStore for GoalSetStore {}
impl GoalSetApiStore for PersistentGoalSetStore {}

/// Execution-owned facade for accepted goals and lifecycle commands.
pub struct GoalSetApi<'a, S> {
    store: &'a mut S,
}

impl<'a, S> GoalSetApi<'a, S>
where
    S: GoalSetApiStore,
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
        self.accept_goal_durable(request)
            .map(|(outcome, _)| outcome)
    }

    /// Validate, commit, flush, and acknowledge one producer-neutral goal request.
    pub fn accept_goal_durable(
        &mut self,
        request: GoalAcceptanceRequest,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&request).map_err(map_store_error)?;
        let command = build_acceptance_add_goal_command(request)?;
        let commit = self
            .store
            .commit_add_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native add-goal command through the same error surface.
    pub fn add_goal(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.add_goal_durable(command).map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one execution-native add command.
    pub fn add_goal_durable(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_add_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native goal replacement through the facade.
    pub fn modify_goal(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.modify_goal_durable(command)
            .map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one goal replacement.
    pub fn modify_goal_durable(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_modify_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native abandonment command through the facade.
    pub fn remove_goal(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.remove_goal_durable(command)
            .map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one abandonment command.
    pub fn remove_goal_durable(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_remove_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native satisfaction command through the facade.
    pub fn satisfy_goal(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.satisfy_goal_durable(command)
            .map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one satisfaction command.
    pub fn satisfy_goal_durable(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_satisfy_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native suspension command through the facade.
    pub fn suspend_goal(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.suspend_goal_durable(command)
            .map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one suspension command.
    pub fn suspend_goal_durable(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_suspend_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    /// Apply an execution-native resume command through the facade.
    pub fn resume_goal(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, GoalSetApiError> {
        self.resume_goal_durable(command)
            .map(|(outcome, _)| outcome)
    }

    /// Commit, flush, and acknowledge one resume command.
    pub fn resume_goal_durable(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        let identity = request_identity(&command).map_err(map_store_error)?;
        let commit = self
            .store
            .commit_resume_goal_command(command, identity)
            .map_err(map_store_error)?;
        self.flush_and_return(commit)
    }

    fn flush_and_return(
        &mut self,
        commit: (GoalCommandOutcome, GoalCommandCommitReceipt),
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), GoalSetApiError> {
        self.store.flush_goal_commands().map_err(map_store_error)?;
        Ok(commit)
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
