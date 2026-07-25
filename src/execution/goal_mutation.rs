//! Cross-domain adapter for agent-authored goal mutations.

use meld_execution::goals::{GoalCommandMetadata, ReopenGoalCommand, SatisfyGoalCommand};
use meld_world_model::{AgentGoalMutationCommand, AgentGoalMutationKind};

/// Request to map one agent-authored mutation into an execution command.
#[derive(Debug, Clone, PartialEq)]
pub struct GoalMutationRequest {
    /// World-model agent mutation command.
    pub command: AgentGoalMutationCommand,
}

/// Errors returned by the agent mutation adapter.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum GoalMutationError {
    /// The world-model mutation command violated its boundary contract.
    #[error("invalid agent goal mutation command: {0}")]
    InvalidCommand(String),
}

/// Execution command mapped from one agent-authored goal mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionGoalMutation {
    /// Mark a goal satisfied under the epoch the review observed.
    Satisfy(SatisfyGoalCommand),
    /// Reopen a satisfied goal in place after belief drift.
    Reopen(ReopenGoalCommand),
}

/// Map an agent-authored mutation into execution's public command surface.
pub fn execution_mutation_from_agent_command(
    request: GoalMutationRequest,
) -> Result<ExecutionGoalMutation, GoalMutationError> {
    let command = request.command;
    command
        .validate()
        .map_err(|err| GoalMutationError::InvalidCommand(err.to_string()))?;
    let metadata = GoalCommandMetadata {
        command_id: command.command_id,
        source_identity: Some(command.dedupe_key.index_key()),
        seq: command.review_seq,
    };
    match command.kind {
        AgentGoalMutationKind::Satisfy {
            at_seq,
            lifecycle_epoch,
        } => Ok(ExecutionGoalMutation::Satisfy(SatisfyGoalCommand {
            metadata,
            goal_id: command.goal_id,
            at_seq,
            lifecycle_epoch,
        })),
        AgentGoalMutationKind::Reopen {
            triggering_belief_revision_id,
            ..
        } => Ok(ExecutionGoalMutation::Reopen(ReopenGoalCommand {
            metadata,
            goal_id: command.goal_id,
            triggering_belief_revision_id,
        })),
    }
}

/// Map an agent-authored satisfaction mutation into execution's public command.
///
/// Compatibility entry for satisfy-only callers; a reopen mutation is a
/// contract violation here and maps through
/// [`execution_mutation_from_agent_command`] instead.
pub fn satisfy_request_from_agent_mutation(
    request: GoalMutationRequest,
) -> Result<SatisfyGoalCommand, GoalMutationError> {
    match execution_mutation_from_agent_command(request)? {
        ExecutionGoalMutation::Satisfy(command) => Ok(command),
        ExecutionGoalMutation::Reopen(_) => Err(GoalMutationError::InvalidCommand(
            "reopen mutation is not a satisfaction command".to_string(),
        )),
    }
}
