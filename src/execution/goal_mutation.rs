//! Cross-domain adapter for agent-authored goal mutations.

use meld_execution::goals::{GoalCommandMetadata, SatisfyGoalCommand};
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

/// Map an agent-authored satisfaction mutation into execution's public command.
pub fn satisfy_request_from_agent_mutation(
    request: GoalMutationRequest,
) -> Result<SatisfyGoalCommand, GoalMutationError> {
    let command = request.command;
    command
        .validate()
        .map_err(|err| GoalMutationError::InvalidCommand(err.to_string()))?;
    match command.kind {
        AgentGoalMutationKind::Satisfy { at_seq } => Ok(SatisfyGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: command.command_id,
                source_identity: Some(command.dedupe_key.index_key()),
                seq: command.review_seq,
            },
            goal_id: command.goal_id,
            at_seq,
        }),
    }
}
