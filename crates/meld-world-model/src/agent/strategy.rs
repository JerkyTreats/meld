//! Agent-owned Strategy judgment and authorization.

use serde::{Deserialize, Serialize};

use crate::agent::{AgentCurationOutcome, AgentDecisionKind};
use crate::strategy::{
    search, verify_candidate, CandidateVerification, StrategyAuthorization, StrategyProblem,
    StrategyRejectionGround, StrategySearchBounds, StrategySearchCompletion, StrategySearchRequest,
};

/// Root-supplied immutable Strategy template for Agent curation ticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStrategyRuntimeConfig {
    /// Problem template whose Goal and planner state are replaced per curation turn.
    pub problem: StrategyProblem,
    /// Explicit structural bounds for the minimal engine.
    pub bounds: StrategySearchBounds,
}

/// Typed failure to authorize a Goal draft through Strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStrategyFailure {
    /// Original curation outcome, retained so the Agent can persist abstention.
    pub outcome: AgentCurationOutcome,
    /// Whether the finite reachable search space was exhausted.
    pub completion: StrategySearchCompletion,
    /// Typed grounds returned by construction or verification.
    pub grounds: Vec<StrategyRejectionGround>,
}

/// Construct, verify, and settle authorization into one curation outcome.
///
/// Strategy recommends and verifies; the Agent remains the authority that
/// attaches the exact candidate to its durable decision and Goal command.
/// Outcomes without a Goal command pass through unchanged.
pub fn authorize_curation_outcome(
    mut outcome: AgentCurationOutcome,
    mut problem: StrategyProblem,
    bounds: StrategySearchBounds,
) -> Result<AgentCurationOutcome, AgentStrategyFailure> {
    let Some(command) = outcome.goal_command.as_mut() else {
        return Ok(outcome);
    };
    problem.goal = command.goal.clone();
    let result = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds,
    });
    let Some(candidate) = result.recommendation else {
        return Err(AgentStrategyFailure {
            outcome,
            completion: result.completion,
            grounds: result.rejections,
        });
    };
    if let CandidateVerification::Invalid { grounds } = verify_candidate(&problem, &candidate) {
        return Err(AgentStrategyFailure {
            outcome,
            completion: result.completion,
            grounds,
        });
    }
    let authorization_id = authorization_identity(
        &outcome.decision.decision_id,
        &candidate.candidate_id,
        &problem.evaluation_policy.policy_id,
    );
    let authorization = StrategyAuthorization {
        authorization_id,
        agent_decision_id: outcome.decision.decision_id.clone(),
        candidate,
        evaluation_policy_id: problem.evaluation_policy.policy_id,
    };
    command.strategy_authorization = Some(authorization.clone());
    outcome.decision.strategy_authorization = Some(authorization);
    outcome.decision.decision = AgentDecisionKind::GoalCommand;
    Ok(outcome)
}

fn authorization_identity(decision_id: &str, candidate_id: &str, policy_id: &str) -> String {
    let bytes = serde_json::to_vec(&(decision_id, candidate_id, policy_id))
        .expect("Strategy authorization identity serialization is infallible");
    format!("strategy-authorization-{}", blake3::hash(&bytes).to_hex())
}
