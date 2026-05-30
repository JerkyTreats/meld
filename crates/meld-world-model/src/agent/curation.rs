//! Pure agent curation and delivery handling.

use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource};

use crate::agent::contracts::{
    deterministic_id, threshold_target, ActiveGoalSummary, AdvanceSubscriptionCommand,
    AgentCurationDecision, AgentCurationDedupeKey, AgentCurationInput, AgentCurationInputRefs,
    AgentCurationOutcome, AgentCurationRuleConfig, AgentDecisionKind, AgentDelivery,
    AgentGoalCommand,
};
use crate::agent::store::AgentStore;
use crate::agent::subscription::AgentSubscription;
use crate::belief::BeliefQuery;
use crate::error::StorageError;
use crate::planner::{PlannerProjectionOutput, PlannerProjectionWarning, PlannerQuery};

/// Coordinates delivery reads, decision persistence, and cursor advancement.
pub struct AgentCuration<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentCuration<'a> {
    /// Create a curation facade over shared agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Handle one subscription delivery and return a persisted outcome when delivered.
    ///
    /// The cursor advances only after the decision write succeeds. Duplicate delivery
    /// returns the stored decision without emitting another command.
    pub fn handle_delivery(
        &self,
        delivery: AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        rule_config: AgentCurationRuleConfig,
    ) -> Result<Option<AgentCurationOutcome>, StorageError> {
        delivery.validate()?;
        let subscription_commands = AgentSubscription::new(self.store);
        if !subscription_commands
            .should_deliver(&delivery.subscription_id, delivery.revision_seq)?
        {
            return Ok(None);
        }
        let input = self.assemble_input(
            &delivery,
            belief_query,
            planner_query,
            active_goals,
            rule_config,
        )?;
        let mut outcome = curate_threshold_rule(input)?;
        let persisted = self.store.put_decision(&outcome.decision)?;
        if persisted != outcome.decision {
            outcome = AgentCurationOutcome {
                decision: persisted,
                goal_command: None,
            };
        }
        subscription_commands.advance_subscription(AdvanceSubscriptionCommand {
            agent_id: delivery.agent_id,
            subscription_id: delivery.subscription_id,
            delivered_revision_id: delivery.belief_revision_id,
            delivered_seq: delivery.revision_seq,
        })?;
        Ok(Some(outcome))
    }

    /// Assemble curation input from public belief and planner facades.
    ///
    /// This validates that the subscription, belief view, perspective, and branch all
    /// belong to the same agent scope before pure curation runs.
    pub fn assemble_input(
        &self,
        delivery: &AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        rule_config: AgentCurationRuleConfig,
    ) -> Result<AgentCurationInput, StorageError> {
        rule_config.validate()?;
        let agent = self.store.get_agent(&delivery.agent_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown agent '{}'", delivery.agent_id))
        })?;
        let subscription = self
            .store
            .get_subscription(&delivery.subscription_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "unknown subscription '{}'",
                    delivery.subscription_id
                ))
            })?;
        if subscription.agent_id != agent.agent_id {
            return Err(StorageError::InvalidPath(
                "subscription agent mismatch".to_string(),
            ));
        }
        if subscription.belief_key.subject != agent.subject {
            return Err(StorageError::InvalidPath(
                "subscription subject mismatch".to_string(),
            ));
        }
        if subscription.belief_key.perspective != agent.perspective_key {
            return Err(StorageError::InvalidPath(
                "subscription perspective mismatch".to_string(),
            ));
        }
        if subscription.belief_key.branch_scope != agent.branch_scope {
            return Err(StorageError::InvalidPath(
                "subscription branch scope mismatch".to_string(),
            ));
        }

        let belief_view = belief_query.current_view(&subscription.belief_key)?;
        let planner_projection = planner_query
            .project_current_world_state(
                &agent.subject,
                &subscription.belief_key.dimension_id,
                Some(agent.perspective_key.clone()),
                Some(agent.branch_scope.clone()),
            )
            .map_err(|err| StorageError::InvalidPath(err.to_string()))?;
        let input_refs = input_refs(
            Some(delivery.belief_revision_id.clone()),
            subscription.belief_key.clone(),
            &planner_projection,
        );
        Ok(AgentCurationInput {
            agent,
            subscription,
            delivered_seq: delivery.revision_seq,
            rule_config,
            belief_view,
            planner_projection,
            active_goals,
            input_refs,
        })
    }
}

/// Run the deterministic threshold rule without writing durable state.
pub fn curate_threshold_rule(
    input: AgentCurationInput,
) -> Result<AgentCurationOutcome, StorageError> {
    input.rule_config.validate()?;
    let dedupe_key = AgentCurationDedupeKey::threshold_rule(
        input.agent.agent_id.clone(),
        &input.agent.subject,
        &input.agent.branch_scope,
        &input.rule_config,
    );
    let revision_id = input.input_refs.belief_revision_id.clone();
    let decision_key = format!("{}::{revision_id:?}", dedupe_key.index_key());
    let decision_id = deterministic_id("decision", &decision_key);

    if projection_blocks_goal(&input.planner_projection) {
        return Ok(absorbed_or_indeterminate(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Indeterminate,
            "planner projection did not provide a usable belief",
            None,
        ));
    }

    let Some(view) = input.belief_view.as_ref() else {
        return Ok(absorbed_or_indeterminate(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Indeterminate,
            "missing belief view",
            None,
        ));
    };
    if view.key.subject != input.agent.subject {
        return Err(StorageError::InvalidPath(
            "belief view subject mismatch".to_string(),
        ));
    }
    if view.perspective != input.agent.perspective_key {
        return Err(StorageError::InvalidPath(
            "belief view perspective mismatch".to_string(),
        ));
    }
    if view.branch_scope != input.agent.branch_scope {
        return Err(StorageError::InvalidPath(
            "belief view branch scope mismatch".to_string(),
        ));
    }
    if view.key.dimension_id != input.rule_config.dimension_id {
        return Ok(absorbed_or_indeterminate(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Absorbed,
            "non matching belief dimension ignored",
            None,
        ));
    }
    if input.active_goals.has_matching_goal(&dedupe_key) {
        return Ok(absorbed_or_indeterminate(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Absorbed,
            "matching active goal already exists",
            None,
        ));
    }
    if view.confidence >= input.rule_config.threshold {
        return Ok(absorbed_or_indeterminate(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Absorbed,
            "belief confidence is sufficient",
            None,
        ));
    }

    let command_key = format!("{}::{revision_id:?}", dedupe_key.index_key());
    let command_id = deterministic_id("goal-command", &command_key);
    let goal_id = deterministic_id("goal", &dedupe_key.index_key());
    let goal = Goal {
        goal_id,
        agent_id: input.agent.agent_id.clone(),
        target: threshold_target(
            input.agent.subject.clone(),
            input.rule_config.dimension_id.clone(),
            input.rule_config.target_condition(),
        ),
        priority: GoalPriority {
            urgency: input.rule_config.priority_urgency,
            cost_ceiling: None,
        },
        source: GoalSource::BeliefDivergence {
            dimension: input.rule_config.dimension_id.clone(),
            observed: format!(
                "{}={}",
                view.planner_projection.confidence_field, view.confidence
            ),
            desired: input.rule_config.desired_summary.clone(),
        },
        lifecycle: GoalLifecycle::Proposed,
    };
    let command = AgentGoalCommand {
        command_id: command_id.clone(),
        goal,
        dedupe_key: dedupe_key.clone(),
    };
    command.validate()?;
    let decision = decision(
        input,
        dedupe_key,
        decision_id,
        AgentDecisionKind::GoalCommand,
        "belief confidence is below threshold",
        Some(command_id),
    );
    Ok(AgentCurationOutcome {
        decision,
        goal_command: Some(command),
    })
}

fn absorbed_or_indeterminate(
    input: AgentCurationInput,
    dedupe_key: AgentCurationDedupeKey,
    decision_id: String,
    kind: AgentDecisionKind,
    reason: &str,
    goal_command_id: Option<String>,
) -> AgentCurationOutcome {
    AgentCurationOutcome {
        decision: decision(
            input,
            dedupe_key,
            decision_id,
            kind,
            reason,
            goal_command_id,
        ),
        goal_command: None,
    }
}

fn decision(
    input: AgentCurationInput,
    dedupe_key: AgentCurationDedupeKey,
    decision_id: String,
    kind: AgentDecisionKind,
    reason: &str,
    goal_command_id: Option<String>,
) -> AgentCurationDecision {
    AgentCurationDecision {
        decision_id,
        agent_id: input.agent.agent_id,
        subscription_id: input.subscription.subscription_id,
        belief_revision_id: input.input_refs.belief_revision_id.clone(),
        belief_key: input.subscription.belief_key,
        projection_version: input.planner_projection.projection_version,
        decision: kind,
        goal_command_id,
        dedupe_key,
        input_refs: input.input_refs,
        reason: reason.to_string(),
        created_at_seq: input.delivered_seq,
    }
}

fn input_refs(
    belief_revision_id: Option<String>,
    belief_key: crate::belief::BeliefKey,
    projection: &PlannerProjectionOutput,
) -> AgentCurationInputRefs {
    AgentCurationInputRefs {
        belief_revision_id,
        belief_key,
        planner_projection_version: projection.projection_version.clone(),
        planner_source_refs: projection
            .source_refs
            .iter()
            .map(|source_ref| format!("{source_ref:?}"))
            .collect(),
        planner_warnings: projection
            .warnings
            .iter()
            .map(|warning| format!("{warning:?}"))
            .collect(),
    }
}

fn projection_blocks_goal(projection: &PlannerProjectionOutput) -> bool {
    projection
        .warnings
        .iter()
        .any(|warning| matches!(warning, PlannerProjectionWarning::MissingBelief { .. }))
}
