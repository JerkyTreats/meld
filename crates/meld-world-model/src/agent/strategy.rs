//! Agent-owned Strategy judgment and authorization.

use meld_events::DomainObjectRef;
use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term, WorldState};
use serde::{Deserialize, Serialize};

use crate::agent::{AgentCurationOutcome, AgentDecisionKind};
use crate::error::StorageError;
use crate::strategy::{
    search, validate_strategy_theory_package, verify_candidate, CandidateVerification,
    StrategyAuthorization, StrategyProblem, StrategyRejectionGround, StrategySearchBounds,
    StrategySearchCompletion, StrategySearchRequest, StrategyTheoryPackage,
};

/// Root-supplied immutable Strategy template for Agent curation ticks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStrategyRuntimeConfig {
    /// Problem template whose Goal and planner state are replaced per curation turn.
    pub problem: StrategyProblem,
    /// Explicit structural bounds for the minimal engine.
    pub bounds: StrategySearchBounds,
    /// Exact installed Strategy theory revision frozen for this composition.
    #[serde(default)]
    pub theory_revision: Option<crate::belief::TheoryRevisionRef>,
}

impl AgentStrategyRuntimeConfig {
    /// Activate one installed Strategy package for a concrete Agent subject.
    ///
    /// The package remains the semantic authority. The supplied subject and
    /// Agent identity are physical grounding inputs and replace the template
    /// Goal during each curation turn.
    pub fn activate_installed(
        package: StrategyTheoryPackage,
        subject: DomainObjectRef,
        agent_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        validate_strategy_theory_package(&package)?;
        let theory_id = package.snapshot.theory_id.clone();
        let goal_pattern = package
            .snapshot
            .settlement_rules
            .first()
            .map(|rule| rule.goal_pattern.clone())
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "Strategy package has no settlement rule for activation".to_string(),
                )
            })?;
        let goal = Goal {
            goal_id: format!("{theory_id}::strategy-template"),
            agent_id: agent_id.into(),
            target: goal_pattern,
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: GoalSource::Maintenance {
                invariant_description: "installed Strategy theory".to_string(),
            },
            lifecycle: GoalLifecycle::Proposed,
        };
        let world_state = WorldState::new(vec![Proposition::Accessible {
            scope: Term::Object(subject),
        }])
        .map_err(|error| {
            StorageError::InvalidPath(format!(
                "installed Strategy activation produced invalid world state: {error:?}"
            ))
        })?;
        Ok(Self {
            problem: StrategyProblem {
                problem_id: format!("{theory_id}::runtime"),
                goal,
                world_state,
                planner_snapshot_id: "runtime-projection".to_string(),
                theory: package.snapshot,
                capabilities: package.capabilities,
                methods: Vec::new(),
                evaluation_policy: package.evaluation_policy,
            },
            bounds: package.search_bounds,
            theory_revision: None,
        })
    }
}

/// Typed failure to authorize a Goal draft through Strategy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentStrategyFailure {
    /// Original curation outcome, retained so the Agent can persist abstention.
    pub outcome: Box<AgentCurationOutcome>,
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
    outcome: AgentCurationOutcome,
    problem: StrategyProblem,
    bounds: StrategySearchBounds,
) -> Result<AgentCurationOutcome, AgentStrategyFailure> {
    authorize_curation_outcome_with_theory(outcome, problem, bounds, None)
}

/// Construct and authorize while pinning the complete Strategy revision.
pub fn authorize_curation_outcome_with_theory(
    mut outcome: AgentCurationOutcome,
    mut problem: StrategyProblem,
    bounds: StrategySearchBounds,
    theory_revision: Option<crate::belief::TheoryRevisionRef>,
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
            outcome: Box::new(outcome),
            completion: result.completion,
            grounds: result.rejections,
        });
    };
    if let CandidateVerification::Invalid { grounds } = verify_candidate(&problem, &candidate) {
        return Err(AgentStrategyFailure {
            outcome: Box::new(outcome),
            completion: result.completion,
            grounds,
        });
    }
    let authorization_id = authorization_identity(
        &outcome.decision.decision_id,
        &candidate.candidate_id,
        &problem.evaluation_policy.policy_id,
        theory_revision.as_ref(),
    );
    let authorization = StrategyAuthorization {
        authorization_id,
        agent_decision_id: outcome.decision.decision_id.clone(),
        candidate,
        evaluation_policy_id: problem.evaluation_policy.policy_id,
        strategy_theory_revision: theory_revision,
    };
    command.strategy_authorization = Some(authorization.clone());
    outcome.decision.strategy_authorization = Some(authorization);
    outcome.decision.decision = AgentDecisionKind::GoalCommand;
    Ok(outcome)
}

fn authorization_identity(
    decision_id: &str,
    candidate_id: &str,
    policy_id: &str,
    theory_revision: Option<&crate::belief::TheoryRevisionRef>,
) -> String {
    let bytes = match theory_revision {
        Some(revision) => serde_json::to_vec(&(decision_id, candidate_id, policy_id, revision)),
        None => serde_json::to_vec(&(decision_id, candidate_id, policy_id)),
    }
    .expect("Strategy authorization identity serialization is infallible");
    format!("strategy-authorization-{}", blake3::hash(&bytes).to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_package_activation_uses_owner_data_without_expression_dispatch() {
        let package: StrategyTheoryPackage = serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/strategy_theory.docs_freshness.json"
        ))
        .unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();

        let activated =
            AgentStrategyRuntimeConfig::activate_installed(package, subject.clone(), "agent-a")
                .unwrap();

        assert_eq!(activated.problem.goal.agent_id, "agent-a");
        assert_eq!(activated.problem.problem_id, "docs_freshness::runtime");
        assert!(activated
            .problem
            .world_state
            .propositions()
            .contains(&Proposition::Accessible {
                scope: Term::Object(subject),
            }));
    }
}
