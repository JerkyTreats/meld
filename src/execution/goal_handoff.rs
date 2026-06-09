//! Handoff from world-model curated goal commands into execution storage.

use meld_execution::goals::{
    AddGoalCommand, GoalCommandMetadata, GoalCommandOutcome, PersistentGoalSetStore,
};
use meld_lang::{GoalLifecycle, Proposition, Term};
use meld_world_model::AgentGoalCommand;
use thiserror::Error;

/// Request to accept one curated agent goal at an execution sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentGoalHandoffRequest {
    /// World-model curation command to accept.
    pub goal_command: AgentGoalCommand,
    /// Sequence assigned by the accepting execution boundary.
    pub accepted_at_seq: u64,
}

/// Errors returned by the curated goal handoff boundary.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentGoalHandoffError {
    /// Curated command or contained goal failed boundary validation.
    #[error("invalid curated goal command: {0}")]
    InvalidCommand(String),
    /// Execution goal storage rejected the accepted command.
    #[error("execution goal store rejected handoff: {0}")]
    Execution(String),
}

/// Convert a world-model curated goal command into an execution add-goal command.
pub fn build_add_goal_command(
    request: &AgentGoalHandoffRequest,
) -> Result<AddGoalCommand, AgentGoalHandoffError> {
    request
        .goal_command
        .validate()
        .map_err(|error| AgentGoalHandoffError::InvalidCommand(error.to_string()))?;

    let goal = &request.goal_command.goal;
    require_non_empty("goal id", &goal.goal_id)?;
    require_non_empty("agent id", &goal.agent_id)?;
    if let Some(variable) = goal.target.grounding_issue() {
        return Err(AgentGoalHandoffError::InvalidCommand(format!(
            "goal target must be ground: {variable}"
        )));
    }
    if !matches!(goal.lifecycle, GoalLifecycle::Proposed) {
        return Err(AgentGoalHandoffError::InvalidCommand(
            "curated goal lifecycle must be proposed".to_string(),
        ));
    }
    require_dedupe_matches_goal(&request.goal_command)?;

    let mut active_goal = goal.clone();
    active_goal.lifecycle = GoalLifecycle::Active;

    Ok(AddGoalCommand {
        metadata: GoalCommandMetadata {
            command_id: request.goal_command.command_id.clone(),
            source_identity: Some(request.goal_command.dedupe_key.index_key()),
            seq: request.accepted_at_seq,
        },
        goal: active_goal,
    })
}

/// Accept one curated agent goal into durable execution goal storage.
pub fn accept_agent_goal_command(
    store: &PersistentGoalSetStore,
    request: AgentGoalHandoffRequest,
) -> Result<GoalCommandOutcome, AgentGoalHandoffError> {
    let command = build_add_goal_command(&request)?;
    store
        .add_goal(command)
        .map_err(|error| AgentGoalHandoffError::Execution(error.to_string()))
}

fn require_non_empty(label: &str, value: &str) -> Result<(), AgentGoalHandoffError> {
    if value.trim().is_empty() {
        return Err(AgentGoalHandoffError::InvalidCommand(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

fn require_dedupe_matches_goal(command: &AgentGoalCommand) -> Result<(), AgentGoalHandoffError> {
    let goal = &command.goal;
    let dedupe_key = &command.dedupe_key;

    if goal.agent_id != dedupe_key.agent_id {
        return dedupe_mismatch("agent id");
    }

    let Proposition::Holds {
        subject,
        dimension,
        condition,
    } = &goal.target
    else {
        return dedupe_mismatch("goal target kind");
    };

    let Term::Object(subject) = subject else {
        return dedupe_mismatch("goal target subject");
    };
    if subject.index_key() != dedupe_key.subject_key {
        return dedupe_mismatch("goal target subject");
    }

    let Term::Dimension(dimension) = dimension else {
        return dedupe_mismatch("goal target dimension");
    };
    if dimension != &dedupe_key.dimension_id {
        return dedupe_mismatch("goal target dimension");
    }

    let condition_key = serde_json::to_string(condition).map_err(|error| {
        AgentGoalHandoffError::InvalidCommand(format!(
            "goal target condition serialization failed: {error}"
        ))
    })?;
    if condition_key != dedupe_key.target_condition_key {
        return dedupe_mismatch("goal target condition");
    }

    Ok(())
}

fn dedupe_mismatch(field: &str) -> Result<(), AgentGoalHandoffError> {
    Err(AgentGoalHandoffError::InvalidCommand(format!(
        "curated goal dedupe key does not match {field}"
    )))
}
