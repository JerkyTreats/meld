//! Pure agent curation and delivery handling.

use meld_lang::{evaluate, Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};

use crate::agent::contracts::{
    condition_key, deterministic_id, threshold_target, ActiveGoalSummary,
    AdvanceSubscriptionCommand, AgentCurationDecision, AgentCurationDedupeKey, AgentCurationInput,
    AgentCurationInputRefs, AgentCurationOutcome, AgentCurationRuleConfig, AgentDecisionKind,
    AgentDelivery, AgentGoalCommand, AgentGoalMutationCommand, AgentGoalMutationKind,
    AgentGoalSatisfactionInput, AgentSatisfactionReview,
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
                goal_mutation_command: None,
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

    /// Handle one satisfaction review and return output only after decision storage.
    ///
    /// This is the supervisor-facing durability barrier for satisfaction curation.
    /// Duplicate reviews return the stored decision. A mutation command is
    /// returned only when the recomputed review matches the persisted decision.
    pub fn handle_satisfaction_review(
        &self,
        review: AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
    ) -> Result<AgentCurationOutcome, StorageError> {
        let input =
            self.assemble_satisfaction_input(&review, belief_query, planner_query, active_goals)?;
        let mut outcome = curate_goal_satisfaction(input)?;
        let persisted = self
            .store
            .put_satisfaction_decision(&review, &outcome.decision)?;
        if persisted != outcome.decision {
            outcome = AgentCurationOutcome {
                decision: persisted,
                goal_command: None,
                goal_mutation_command: None,
            };
        }
        Ok(outcome)
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

    /// Assemble satisfaction input from public belief and planner facades.
    ///
    /// The agent domain owns the scope checks here so runtime assembly can
    /// supply stores and active goals without becoming satisfaction authority.
    pub fn assemble_satisfaction_input(
        &self,
        review: &AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
    ) -> Result<AgentGoalSatisfactionInput, StorageError> {
        review.validate()?;
        let agent = self.store.get_agent(&review.agent_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown agent '{}'", review.agent_id))
        })?;
        let subscription = self
            .store
            .get_subscription(&review.subscription_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "unknown subscription '{}'",
                    review.subscription_id
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
            belief_view
                .as_ref()
                .and_then(|view| view.current_revision_id.clone()),
            subscription.belief_key.clone(),
            &planner_projection,
        );
        Ok(AgentGoalSatisfactionInput {
            agent,
            subscription,
            review_seq: review.review_seq,
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
    if view.key.perspective != input.agent.perspective_key {
        return Err(StorageError::InvalidPath(
            "belief view perspective mismatch".to_string(),
        ));
    }
    if view.key.branch_scope != input.agent.branch_scope {
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
    let confidence = view.planner_projection.confidence;
    if confidence >= input.rule_config.threshold {
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
                view.planner_projection.confidence_field, confidence
            ),
            desired: input.rule_config.desired_summary.clone(),
        },
        lifecycle: GoalLifecycle::Proposed,
    };
    let command = AgentGoalCommand {
        command_id: command_id.clone(),
        goal,
        dedupe_key: dedupe_key.clone(),
        strategy_authorization: None,
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
        goal_mutation_command: None,
    })
}

/// Run deterministic agent-owned satisfaction curation without writing durable state.
///
/// Precedence over the agent's goals, in deterministic goal id order:
/// an active goal whose target is satisfied emits `Satisfy` carrying the
/// observed lifecycle epoch; otherwise a satisfied goal whose maintained
/// condition the reviewed revision violates emits an idempotent `Reopen`
/// citing that revision (the frozen future-drift rule — satisfied is never
/// absorbing); otherwise the review is absorbed or indeterminate.
pub fn curate_goal_satisfaction(
    input: AgentGoalSatisfactionInput,
) -> Result<AgentCurationOutcome, StorageError> {
    validate_satisfaction_input(&input)?;
    let mut owned: Vec<&Goal> = input
        .active_goals
        .goals
        .iter()
        .filter(|goal| goal.agent_id == input.agent.agent_id)
        .filter(|goal| {
            matches!(
                goal.lifecycle,
                GoalLifecycle::Active | GoalLifecycle::Satisfied { .. }
            )
        })
        .collect();
    owned.sort_by(|left, right| left.goal_id.cmp(&right.goal_id));

    if owned.is_empty() {
        let dedupe_key = no_candidate_dedupe_key(&input);
        let decision_key = format!(
            "{}::satisfy::none::{}",
            dedupe_key.index_key(),
            input.review_seq
        );
        let decision_id = deterministic_id("decision", &decision_key);
        return Ok(satisfaction_without_command(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Absorbed,
            "no active goals for agent",
        ));
    }

    let mut first_satisfied: Option<&Goal> = None;
    let mut satisfied_count = 0usize;
    let mut first_unsatisfied: Option<&Goal> = None;
    let mut first_indeterminate: Option<&Goal> = None;
    // First satisfied-lifecycle goal whose maintained condition the reviewed
    // state violates, captured as owned identity so the input can move later.
    let mut first_drifted: Option<(String, AgentCurationDedupeKey)> = None;
    let mut first_stable: Option<(String, AgentCurationDedupeKey)> = None;

    for goal in &owned {
        if let Some(variable) = goal.target.grounding_issue() {
            return Err(StorageError::InvalidPath(format!(
                "candidate goal target must be ground: {variable}"
            )));
        }

        let evaluation = evaluate(&input.planner_projection.world_state, &goal.target);
        if matches!(goal.lifecycle, GoalLifecycle::Satisfied { .. }) {
            match evaluation {
                meld_lang::EvalResult::Unsatisfied { .. } => {
                    if first_drifted.is_none() {
                        first_drifted = Some((
                            goal.goal_id.clone(),
                            dedupe_key_for_goal(&input.agent, goal)?,
                        ));
                    }
                }
                _ => {
                    if first_stable.is_none() {
                        first_stable = Some((
                            goal.goal_id.clone(),
                            dedupe_key_for_goal(&input.agent, goal)?,
                        ));
                    }
                }
            }
            continue;
        }

        match evaluation {
            meld_lang::EvalResult::Satisfied => {
                if first_satisfied.is_none() {
                    first_satisfied = Some(goal);
                }
                satisfied_count += 1;
            }
            meld_lang::EvalResult::Unsatisfied { .. } => {
                if first_unsatisfied.is_none() {
                    first_unsatisfied = Some(goal);
                }
            }
            meld_lang::EvalResult::Indeterminate { .. } => {
                if first_indeterminate.is_none() {
                    first_indeterminate = Some(goal);
                }
            }
        }
    }

    if let Some(goal) = first_satisfied {
        let dedupe_key = dedupe_key_for_goal(&input.agent, goal)?;
        let decision_key = format!(
            "{}::satisfy::{}::{}",
            dedupe_key.index_key(),
            goal.goal_id,
            input.review_seq
        );
        let decision_id = deterministic_id("decision", &decision_key);
        let command_id = deterministic_id("goal-mutation-command", &decision_key);
        let command = AgentGoalMutationCommand {
            command_id: command_id.clone(),
            agent_id: input.agent.agent_id.clone(),
            goal_id: goal.goal_id.clone(),
            kind: AgentGoalMutationKind::Satisfy {
                at_seq: input.review_seq,
                lifecycle_epoch: input.active_goals.lifecycle_epoch(&goal.goal_id),
            },
            dedupe_key: dedupe_key.clone(),
            review_seq: input.review_seq,
            projection_version: input.planner_projection.projection_version.clone(),
            planner_source_refs: input.input_refs.planner_source_refs.clone(),
            planner_warnings: input.input_refs.planner_warnings.clone(),
        };
        command.validate()?;
        let reason = if satisfied_count > 1 {
            "first satisfied goal was selected"
        } else {
            "goal target satisfied"
        };
        let decision = satisfaction_decision(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::GoalMutationCommand,
            reason,
            Some(command_id),
        );
        return Ok(AgentCurationOutcome {
            decision,
            goal_command: None,
            goal_mutation_command: Some(command),
        });
    }

    if let Some((goal_id, dedupe_key)) = first_drifted {
        return curate_reopen_for_drift(input, goal_id, dedupe_key);
    }

    if let Some(goal) = first_indeterminate {
        let dedupe_key = dedupe_key_for_goal(&input.agent, goal)?;
        let decision_key = format!(
            "{}::satisfy::{}::{}",
            dedupe_key.index_key(),
            goal.goal_id,
            input.review_seq
        );
        let decision_id = deterministic_id("decision", &decision_key);
        return Ok(satisfaction_without_command(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Indeterminate,
            "goal target indeterminate",
        ));
    }

    if let Some(goal) = first_unsatisfied {
        let dedupe_key = dedupe_key_for_goal(&input.agent, goal)?;
        let decision_key = format!(
            "{}::satisfy::{}::{}",
            dedupe_key.index_key(),
            goal.goal_id,
            input.review_seq
        );
        let decision_id = deterministic_id("decision", &decision_key);
        return Ok(satisfaction_without_command(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Absorbed,
            "goal target unsatisfied",
        ));
    }

    let (goal_id, dedupe_key) = first_stable.expect("owned goal list is non-empty");
    let decision_key = format!(
        "{}::satisfied-stable::{}::{}",
        dedupe_key.index_key(),
        goal_id,
        input.review_seq
    );
    let decision_id = deterministic_id("decision", &decision_key);
    Ok(satisfaction_without_command(
        input,
        dedupe_key,
        decision_id,
        AgentDecisionKind::Absorbed,
        "satisfied goals show no drift",
    ))
}

/// Build the idempotent reopen outcome for one drifted satisfied goal.
///
/// The command identity derives from the goal, the triggering revision, and
/// the observed epoch — not the review sequence — so a replayed reopen for
/// the same drift is byte-identical and execution absorbs it as a duplicate.
fn curate_reopen_for_drift(
    input: AgentGoalSatisfactionInput,
    goal_id: String,
    dedupe_key: AgentCurationDedupeKey,
) -> Result<AgentCurationOutcome, StorageError> {
    let decision_key = format!(
        "{}::reopen::{}::{}",
        dedupe_key.index_key(),
        goal_id,
        input.review_seq
    );
    let decision_id = deterministic_id("decision", &decision_key);
    let Some(revision_id) = input.input_refs.belief_revision_id.clone() else {
        // Drift provenance must cite the triggering revision; without one
        // the review cannot author a well-formed reopen.
        return Ok(satisfaction_without_command(
            input,
            dedupe_key,
            decision_id,
            AgentDecisionKind::Indeterminate,
            "drift review has no belief revision to cite",
        ));
    };
    let observed_epoch = input.active_goals.lifecycle_epoch(&goal_id);
    let command_key = format!(
        "{}::reopen::{}::{}::{}",
        dedupe_key.index_key(),
        goal_id,
        revision_id,
        observed_epoch
    );
    let command_id = deterministic_id("goal-mutation-command", &command_key);
    let command = AgentGoalMutationCommand {
        command_id: command_id.clone(),
        agent_id: input.agent.agent_id.clone(),
        goal_id,
        kind: AgentGoalMutationKind::Reopen {
            triggering_belief_revision_id: revision_id,
            observed_lifecycle_epoch: observed_epoch,
        },
        dedupe_key: dedupe_key.clone(),
        review_seq: input.review_seq,
        projection_version: input.planner_projection.projection_version.clone(),
        planner_source_refs: input.input_refs.planner_source_refs.clone(),
        planner_warnings: input.input_refs.planner_warnings.clone(),
    };
    command.validate()?;
    let decision = satisfaction_decision(
        input,
        dedupe_key,
        decision_id,
        AgentDecisionKind::GoalMutationCommand,
        "satisfied goal drifted below its maintained condition",
        Some(command_id),
    );
    Ok(AgentCurationOutcome {
        decision,
        goal_command: None,
        goal_mutation_command: Some(command),
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
        goal_mutation_command: None,
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
        decision: kind,
        goal_command_id,
        goal_mutation_command_id: None,
        strategy_authorization: None,
        dedupe_key,
        input_refs: input.input_refs,
        reason: reason.to_string(),
        created_at_seq: input.delivered_seq,
    }
}

fn satisfaction_without_command(
    input: AgentGoalSatisfactionInput,
    dedupe_key: AgentCurationDedupeKey,
    decision_id: String,
    kind: AgentDecisionKind,
    reason: &str,
) -> AgentCurationOutcome {
    AgentCurationOutcome {
        decision: satisfaction_decision(input, dedupe_key, decision_id, kind, reason, None),
        goal_command: None,
        goal_mutation_command: None,
    }
}

fn satisfaction_decision(
    input: AgentGoalSatisfactionInput,
    dedupe_key: AgentCurationDedupeKey,
    decision_id: String,
    kind: AgentDecisionKind,
    reason: &str,
    goal_mutation_command_id: Option<String>,
) -> AgentCurationDecision {
    AgentCurationDecision {
        decision_id,
        agent_id: input.agent.agent_id,
        subscription_id: input.subscription.subscription_id,
        decision: kind,
        goal_command_id: None,
        goal_mutation_command_id,
        strategy_authorization: None,
        dedupe_key,
        input_refs: input.input_refs,
        reason: reason.to_string(),
        created_at_seq: input.review_seq,
    }
}

fn validate_satisfaction_input(input: &AgentGoalSatisfactionInput) -> Result<(), StorageError> {
    if input.review_seq == 0 {
        return Err(StorageError::InvalidPath(
            "review seq must be greater than zero".to_string(),
        ));
    }
    input.agent.validate()?;
    input.subscription.validate()?;
    if input.subscription.agent_id != input.agent.agent_id {
        return Err(StorageError::InvalidPath(
            "subscription agent mismatch".to_string(),
        ));
    }
    input.input_refs.validate()?;
    if input.planner_projection.projection_version != input.input_refs.planner_projection_version {
        return Err(StorageError::InvalidPath(
            "planner projection version mismatch".to_string(),
        ));
    }
    Ok(())
}

fn no_candidate_dedupe_key(input: &AgentGoalSatisfactionInput) -> AgentCurationDedupeKey {
    AgentCurationDedupeKey {
        agent_id: input.agent.agent_id.clone(),
        subject_key: input.agent.subject.index_key(),
        branch_id: input.agent.branch_scope.branch_id.clone(),
        dimension_id: input.subscription.belief_key.dimension_id.clone(),
        target_condition_key: "no-active-goal".to_string(),
        source_kind: "goal_satisfaction_review".to_string(),
    }
}

fn dedupe_key_for_goal(
    agent: &crate::agent::contracts::AgentRecord,
    goal: &Goal,
) -> Result<AgentCurationDedupeKey, StorageError> {
    let target_key = serde_json::to_string(&goal.target)
        .map_err(|err| StorageError::InvalidPath(err.to_string()))?;
    let (subject_key, dimension_id, target_condition_key) = match &goal.target {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => (
            target_subject_key(subject),
            target_dimension_key(dimension),
            condition_key(condition),
        ),
        _ => (
            target_key.clone(),
            "proposition".to_string(),
            target_key.clone(),
        ),
    };

    Ok(AgentCurationDedupeKey {
        agent_id: agent.agent_id.clone(),
        subject_key,
        branch_id: agent.branch_scope.branch_id.clone(),
        dimension_id,
        target_condition_key,
        source_kind: goal_source_kind(&goal.source).to_string(),
    })
}

fn target_subject_key(subject: &Term) -> String {
    match subject {
        Term::Object(object) => object.index_key(),
        _ => serde_json::to_string(subject).expect("term serialization is infallible"),
    }
}

fn target_dimension_key(dimension: &Term) -> String {
    match dimension {
        Term::Dimension(dimension) => dimension.clone(),
        _ => serde_json::to_string(dimension).expect("term serialization is infallible"),
    }
}

fn goal_source_kind(source: &GoalSource) -> &'static str {
    match source {
        GoalSource::BeliefDivergence { .. } => "belief_divergence",
        GoalSource::UserDirected { .. } => "user_directed",
        GoalSource::Maintenance { .. } => "maintenance",
        GoalSource::Decomposed { .. } => "decomposed",
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
