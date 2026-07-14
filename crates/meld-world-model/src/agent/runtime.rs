//! Agent-owned runtime facades for durable curation ticks.
//!
//! These facades keep runtime assembly thin: callers provide public belief and
//! planner query ports plus execution-owned sink ports, while the agent domain
//! owns curation persistence and subscription cursor ordering.

use crate::agent::contracts::{
    ActiveGoalSummary, AdvanceSubscriptionCommand, AgentAuthoredCommand, AgentCurationDecision,
    AgentCurationOutcome, AgentCurationRuleConfig, AgentDecisionKind, AgentDelivery,
    AgentDeliverySelection, AgentGoalCommand, AgentGoalMutationCommand, AgentGoalRecoverySelection,
    AgentSatisfactionCursorCasIntent, AgentSatisfactionRecoverySelection, AgentSatisfactionReview,
    AgentSatisfactionReviewSelection, AgentSinkReceipt, AgentSinkReceiptKind, AgentSinkSubmission,
};
use crate::agent::curation::{curate_goal_satisfaction, curate_threshold_rule, AgentCuration};
use crate::agent::selection::AgentSemanticSelector;
use crate::agent::store::AgentStore;
use crate::agent::subscription::AgentSubscription;
use crate::belief::BeliefQuery;
use crate::error::StorageError;
use crate::planner::PlannerQuery;

/// Error returned by an execution-owned sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSinkError {
    /// Human-readable error for runtime diagnostics.
    pub message: String,
    /// Whether the runtime should classify the failure as retryable.
    pub retryable: bool,
}

/// Error returned by an execution-owned active goal query port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentActiveGoalQueryError {
    /// Human-readable error for runtime diagnostics.
    pub message: String,
    /// Whether the runtime should classify the failure as retryable.
    pub retryable: bool,
}

impl AgentActiveGoalQueryError {
    /// Create a retryable active goal query error.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    /// Create a fatal active goal query error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

/// Execution-owned port for loading the latest active goal view.
pub trait AgentActiveGoalQuery {
    /// Load the latest goals visible to the agent immediately before curation.
    fn active_goals_for_agent(
        &mut self,
        agent_id: &str,
    ) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError>;
}

impl<F> AgentActiveGoalQuery for F
where
    F: FnMut(&str) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError>,
{
    fn active_goals_for_agent(
        &mut self,
        agent_id: &str,
    ) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError> {
        self(agent_id)
    }
}

/// Error returned by an execution-owned durable command outcome query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommandOutcomeQueryError {
    /// Human-readable error for runtime diagnostics.
    pub message: String,
    /// Whether the runtime should classify the failure as retryable.
    pub retryable: bool,
}

impl AgentCommandOutcomeQueryError {
    /// Create a retryable command outcome query error.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    /// Create a fatal command outcome query error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

/// Execution-owned port for recovering a durable command acceptance outcome.
pub trait AgentCommandOutcomeQuery {
    /// Return the committed submission for one exact decision and command.
    fn committed_submission(
        &mut self,
        decision: &AgentCurationDecision,
        command: &AgentAuthoredCommand,
    ) -> Result<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>;
}

impl<F> AgentCommandOutcomeQuery for F
where
    F: FnMut(
        &AgentCurationDecision,
        &AgentAuthoredCommand,
    ) -> Result<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>,
{
    fn committed_submission(
        &mut self,
        decision: &AgentCurationDecision,
        command: &AgentAuthoredCommand,
    ) -> Result<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError> {
        self(decision, command)
    }
}

impl AgentSinkError {
    /// Create a retryable sink error.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    /// Create a fatal sink error.
    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

/// Execution-owned port for submitting proposed goal commands.
pub trait AgentGoalCommandSink {
    /// Submit one idempotent goal command to execution.
    fn submit_goal_command(
        &mut self,
        command: &AgentGoalCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError>;
}

impl<F> AgentGoalCommandSink for F
where
    F: FnMut(&AgentGoalCommand) -> Result<AgentSinkSubmission, AgentSinkError>,
{
    fn submit_goal_command(
        &mut self,
        command: &AgentGoalCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self(command)
    }
}

/// Execution-owned port for submitting goal lifecycle mutation commands.
pub trait AgentGoalMutationSink {
    /// Submit one idempotent goal mutation command to execution.
    fn submit_goal_mutation(
        &mut self,
        command: &AgentGoalMutationCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError>;
}

impl<F> AgentGoalMutationSink for F
where
    F: FnMut(&AgentGoalMutationCommand) -> Result<AgentSinkSubmission, AgentSinkError>,
{
    fn submit_goal_mutation(
        &mut self,
        command: &AgentGoalMutationCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self(command)
    }
}

/// Bounded diagnostic report returned by one agent runtime tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRuntimeReport {
    /// Agent actor that owned the tick input.
    pub actor_id: String,
    /// Input sequence supplied by the delivery or review.
    pub input_sequence: u64,
    /// Last durable output sequence observed or advanced by the tick.
    pub output_sequence: u64,
    /// Number of deliveries or reviews accepted for curation.
    pub delivered_count: usize,
    /// Number of curation decisions persisted or reused from durable storage.
    pub decision_count: usize,
    /// Number of sink submissions attempted.
    pub sink_submission_count: usize,
    /// Durable sink receipts observed or recorded by the tick.
    pub sink_receipts: Vec<AgentSinkReceipt>,
    /// Retryable errors encountered before the tick reached a durable boundary.
    pub retryable_errors: Vec<String>,
    /// Fatal errors encountered before the tick reached a durable boundary.
    pub fatal_errors: Vec<String>,
    /// Whether the bounded tick stopped because its budget was exhausted.
    pub budget_exhausted: bool,
}

impl AgentRuntimeReport {
    fn new(actor_id: impl Into<String>, input_sequence: u64, output_sequence: u64) -> Self {
        Self {
            actor_id: actor_id.into(),
            input_sequence,
            output_sequence,
            delivered_count: 0,
            decision_count: 0,
            sink_submission_count: 0,
            sink_receipts: Vec::new(),
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }

    fn retryable_error(&mut self, error: impl Into<String>) {
        self.retryable_errors.push(error.into());
    }

    fn fatal_error(&mut self, error: impl Into<String>) {
        self.fatal_errors.push(error.into());
    }
}

/// Runtime facade for agent goal curation deliveries.
pub struct AgentGoalCurationRuntime<'a> {
    store: &'a AgentStore,
}

/// Immutable inputs for one selector-issued goal curation tick.
pub struct AgentSelectedGoalTick<'a> {
    /// Exact selector token for the pending belief revision.
    pub selection: AgentDeliverySelection,
    /// Public belief query bound to the current world-model authority.
    pub belief_query: &'a BeliefQuery<'a>,
    /// Public planner projection query used by pure curation.
    pub planner_query: &'a PlannerQuery<'a>,
    /// Durable rule configuration selected for the agent.
    pub rule_config: AgentCurationRuleConfig,
}

impl<'a> AgentGoalCurationRuntime<'a> {
    /// Bind the runtime facade to durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Curate one delivery, submit any goal command, and advance the cursor.
    ///
    /// This compatibility entrypoint can persist command-free decisions only.
    /// Command-producing work requires `handle_selected_delivery` so the final
    /// durable commit carries the selector-issued owner and belief fences.
    pub fn handle_delivery<S>(
        &self,
        delivery: AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        rule_config: AgentCurationRuleConfig,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        S: AgentGoalCommandSink,
    {
        let mut goal_query = |_agent_id: &str| {
            Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(active_goals.clone())
        };
        self.handle_delivery_core(
            delivery,
            belief_query,
            planner_query,
            &mut goal_query,
            rule_config,
            sink,
        )
    }

    /// Curate one delivery after reloading active goals from execution.
    ///
    /// This compatibility entrypoint cannot emit a goal command. Recurring
    /// semantic actors must use `handle_selected_delivery`.
    pub fn handle_delivery_with_goal_query<Q, S>(
        &self,
        delivery: AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        goal_query: &mut Q,
        rule_config: AgentCurationRuleConfig,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        S: AgentGoalCommandSink,
    {
        self.handle_delivery_core(
            delivery,
            belief_query,
            planner_query,
            goal_query,
            rule_config,
            sink,
        )
    }

    /// Curate one selector-issued delivery with exact recovery and cursor fences.
    pub fn handle_selected_delivery<Q, O, S>(
        &self,
        tick: AgentSelectedGoalTick<'_>,
        goal_query: &mut Q,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        O: AgentCommandOutcomeQuery,
        S: AgentGoalCommandSink,
    {
        let selection = tick.selection;
        let delivery = selection.delivery.clone();
        let mut report =
            AgentRuntimeReport::new(delivery.agent_id.clone(), delivery.revision_seq, 0);
        let selector = AgentSemanticSelector::new(self.store);
        if let Err(error) = selector.revalidate_delivery(&selection, tick.belief_query) {
            push_storage_error(&mut report, error);
            return report;
        }
        match self.store.outcome_for_selected_delivery(&selection) {
            Ok(Some(outcome)) => {
                report.delivered_count = 1;
                report.decision_count = 1;
                return self.finish_selected_delivery(
                    &selection,
                    &outcome,
                    None,
                    outcome_query,
                    sink,
                    report,
                );
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        }
        let active_goals = match goal_query.active_goals_for_agent(&delivery.agent_id) {
            Ok(active_goals) => active_goals,
            Err(error) => {
                push_goal_query_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        let curation = AgentCuration::new(self.store);
        let input = match curation.assemble_input(
            &delivery,
            tick.belief_query,
            tick.planner_query,
            active_goals,
            tick.rule_config,
        ) {
            Ok(input) => input,
            Err(error) => {
                report.fatal_error(error.to_string());
                return report;
            }
        };
        let outcome = match curate_threshold_rule(input) {
            Ok(outcome) => outcome,
            Err(error) => {
                report.fatal_error(error.to_string());
                return report;
            }
        };
        // Pure cross-domain reads may take time, so the selector token is checked
        // again immediately before the atomic owner-fenced decision commit.
        if let Err(error) = selector.revalidate_delivery(&selection, tick.belief_query) {
            push_storage_error(&mut report, error);
            return report;
        }
        let outcome = match self
            .store
            .put_selected_delivery_outcome(&selection, &outcome)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        };
        report.decision_count = 1;
        self.finish_selected_delivery(&selection, &outcome, None, outcome_query, sink, report)
    }

    /// Complete one exact historical outcome without consulting the current belief head.
    pub fn handle_recovered_delivery<O, S>(
        &self,
        recovery: AgentGoalRecoverySelection,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalCommandSink,
    {
        let mut report = AgentRuntimeReport::new(
            recovery.selection.delivery.agent_id.clone(),
            recovery.selection.delivery.revision_seq,
            0,
        );
        let outcome = match self.store.goal_recovery_outcome(&recovery) {
            Ok(outcome) => outcome,
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        report.decision_count = 1;
        self.finish_selected_delivery(
            &recovery.selection,
            &outcome,
            Some(&recovery),
            outcome_query,
            sink,
            report,
        )
    }

    fn finish_selected_delivery<O, S>(
        &self,
        selection: &AgentDeliverySelection,
        outcome: &AgentCurationOutcome,
        recovery: Option<&AgentGoalRecoverySelection>,
        outcome_query: &mut O,
        sink: &mut S,
        mut report: AgentRuntimeReport,
    ) -> AgentRuntimeReport
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalCommandSink,
    {
        if !self.complete_selected_goal_command(outcome, recovery, outcome_query, sink, &mut report)
        {
            return report;
        }
        match self
            .store
            .advance_selected_delivery_cursor(selection, &outcome.decision.decision_id)
        {
            Ok(updated) => report.output_sequence = updated.last_delivered_seq,
            Err(error) => push_storage_error(&mut report, error),
        }
        report
    }

    fn complete_selected_goal_command<O, S>(
        &self,
        outcome: &AgentCurationOutcome,
        recovery: Option<&AgentGoalRecoverySelection>,
        outcome_query: &mut O,
        sink: &mut S,
        report: &mut AgentRuntimeReport,
    ) -> bool
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalCommandSink,
    {
        match existing_sink_receipt(self.store, &outcome.decision) {
            Ok(Some(receipt)) => {
                report.sink_receipts.push(receipt);
                return true;
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(report, error);
                return false;
            }
        }
        let Some(command) = &outcome.goal_command else {
            if outcome.decision.decision == AgentDecisionKind::GoalCommand {
                report.fatal_error("durable goal decision has no exact command outbox");
                return false;
            }
            return true;
        };
        let authored = AgentAuthoredCommand::Goal(Box::new(command.clone()));
        match outcome_query.committed_submission(&outcome.decision, &authored) {
            Ok(Some(submission)) => {
                if !self.revalidate_goal_recovery(recovery, report) {
                    return false;
                }
                return record_sink_receipt(
                    self.store,
                    &outcome.decision,
                    AgentSinkReceiptKind::GoalCommand,
                    submission,
                    &command.command_id,
                    report,
                );
            }
            Ok(None) => {}
            Err(error) => {
                push_outcome_query_error(report, error);
                return false;
            }
        }
        if !self.revalidate_goal_recovery(recovery, report) {
            return false;
        }
        report.sink_submission_count += 1;
        match sink.submit_goal_command(command) {
            Ok(submission) => record_sink_receipt(
                self.store,
                &outcome.decision,
                AgentSinkReceiptKind::GoalCommand,
                submission,
                &command.command_id,
                report,
            ),
            Err(error) => {
                push_sink_error(report, error);
                false
            }
        }
    }

    fn revalidate_goal_recovery(
        &self,
        recovery: Option<&AgentGoalRecoverySelection>,
        report: &mut AgentRuntimeReport,
    ) -> bool {
        let Some(recovery) = recovery else {
            return true;
        };
        match self.store.goal_recovery_outcome(recovery) {
            Ok(_) => true,
            Err(error) => {
                push_storage_error(report, error);
                false
            }
        }
    }

    fn handle_delivery_core<Q, S>(
        &self,
        delivery: AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        goal_query: &mut Q,
        rule_config: AgentCurationRuleConfig,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        S: AgentGoalCommandSink,
    {
        let mut report =
            AgentRuntimeReport::new(delivery.agent_id.clone(), delivery.revision_seq, 0);
        if let Err(error) = delivery.validate() {
            report.fatal_error(error.to_string());
            return report;
        }

        let subscriptions = AgentSubscription::new(self.store);
        let Some(subscription) = self.read_subscription(&delivery, &mut report) else {
            return report;
        };
        report.output_sequence = subscription.last_delivered_seq;

        let should_deliver =
            match subscriptions.should_deliver(&delivery.subscription_id, delivery.revision_seq) {
                Ok(should_deliver) => should_deliver,
                Err(error) => {
                    report.fatal_error(error.to_string());
                    return report;
                }
            };
        if !should_deliver {
            return report;
        }

        let active_goals = match goal_query.active_goals_for_agent(&delivery.agent_id) {
            Ok(active_goals) => active_goals,
            Err(error) => {
                push_goal_query_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        let Some(outcome) = self.persist_goal_decision(
            delivery.clone(),
            belief_query,
            planner_query,
            active_goals.clone(),
            rule_config,
            &mut report,
        ) else {
            return report;
        };

        if !self.submit_goal_command_if_needed(&outcome, &active_goals, sink, &mut report) {
            return report;
        }

        match subscriptions.advance_subscription(AdvanceSubscriptionCommand {
            agent_id: delivery.agent_id,
            subscription_id: delivery.subscription_id,
            delivered_revision_id: delivery.belief_revision_id,
            delivered_seq: delivery.revision_seq,
        }) {
            Ok(updated) => {
                if let Err(error) = self.store.flush() {
                    report.fatal_error(error.to_string());
                    return report;
                }
                report.output_sequence = updated.last_delivered_seq;
            }
            Err(error) => report.fatal_error(error.to_string()),
        }
        report
    }

    fn read_subscription(
        &self,
        delivery: &AgentDelivery,
        report: &mut AgentRuntimeReport,
    ) -> Option<crate::agent::contracts::AgentSubscriptionRecord> {
        match self.store.get_subscription(&delivery.subscription_id) {
            Ok(Some(subscription)) => {
                if subscription.agent_id != delivery.agent_id {
                    report.fatal_error("subscription agent mismatch");
                    None
                } else {
                    Some(subscription)
                }
            }
            Ok(None) => {
                report.fatal_error(format!(
                    "unknown subscription '{}'",
                    delivery.subscription_id
                ));
                None
            }
            Err(error) => {
                report.fatal_error(error.to_string());
                None
            }
        }
    }

    fn persist_goal_decision(
        &self,
        delivery: AgentDelivery,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        rule_config: AgentCurationRuleConfig,
        report: &mut AgentRuntimeReport,
    ) -> Option<AgentCurationOutcome> {
        let curation = AgentCuration::new(self.store);
        let input = match curation.assemble_input(
            &delivery,
            belief_query,
            planner_query,
            active_goals,
            rule_config,
        ) {
            Ok(input) => input,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        let outcome = match curate_threshold_rule(input) {
            Ok(outcome) => outcome,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        match self.store.decision_by_dedupe_and_revision(
            &outcome.decision.dedupe_key,
            outcome.decision.input_refs.belief_revision_id.as_deref(),
        ) {
            Ok(Some(existing)) => {
                report.decision_count = 1;
                return match self.store.decision_outbox(&existing.decision_id) {
                    Ok(Some(_)) => match self.store.outcome_for_decision(&existing.decision_id) {
                        Ok(outcome) => Some(outcome),
                        Err(error) => {
                            report.fatal_error(error.to_string());
                            None
                        }
                    },
                    Ok(None) => Some(AgentCurationOutcome {
                        decision: existing,
                        goal_command: None,
                        goal_mutation_command: None,
                    }),
                    Err(error) => {
                        report.fatal_error(error.to_string());
                        None
                    }
                };
            }
            Ok(None) => {}
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        }
        let persisted = match self.store.put_decision(&outcome.decision) {
            Ok(persisted) => persisted,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        report.decision_count = 1;
        if persisted == outcome.decision {
            Some(outcome)
        } else {
            Some(AgentCurationOutcome {
                decision: persisted,
                goal_command: None,
                goal_mutation_command: None,
            })
        }
    }

    fn submit_goal_command_if_needed<S>(
        &self,
        outcome: &AgentCurationOutcome,
        _active_goals: &ActiveGoalSummary,
        sink: &mut S,
        report: &mut AgentRuntimeReport,
    ) -> bool
    where
        S: AgentGoalCommandSink,
    {
        match existing_sink_receipt(self.store, &outcome.decision) {
            Ok(Some(receipt)) => {
                report.sink_receipts.push(receipt);
                return true;
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(report, error);
                return false;
            }
        }

        if outcome.decision.decision == AgentDecisionKind::GoalCommand {
            report.fatal_error(
                "compatibility goal curation cannot emit commands without a selector-issued owner fence",
            );
            return false;
        }
        let Some(command) = &outcome.goal_command else {
            return true;
        };

        report.sink_submission_count += 1;
        match sink.submit_goal_command(command) {
            Ok(submission) => record_sink_receipt(
                self.store,
                &outcome.decision,
                AgentSinkReceiptKind::GoalCommand,
                submission,
                &command.command_id,
                report,
            ),
            Err(error) => {
                push_sink_error(report, error);
                false
            }
        }
    }
}

/// Runtime facade for agent-owned satisfaction reviews.
pub struct AgentSatisfactionCurationRuntime<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentSatisfactionCurationRuntime<'a> {
    /// Bind the runtime facade to durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Curate one satisfaction review and submit any mutation command.
    ///
    /// This compatibility entrypoint can persist command-free decisions only.
    /// Command-producing work requires `handle_selected_review` so the final
    /// durable commit carries the selector-issued owner and belief fences.
    pub fn handle_review<S>(
        &self,
        review: AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        S: AgentGoalMutationSink,
    {
        let mut goal_query = |_agent_id: &str| {
            Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(active_goals.clone())
        };
        self.handle_review_core(review, belief_query, planner_query, &mut goal_query, sink)
    }

    /// Curate one satisfaction review after reloading active goals from execution.
    ///
    /// This compatibility entrypoint cannot emit a goal mutation. Recurring
    /// semantic actors must use `handle_selected_review`.
    pub fn handle_review_with_goal_query<Q, S>(
        &self,
        review: AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        goal_query: &mut Q,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        S: AgentGoalMutationSink,
    {
        self.handle_review_core(review, belief_query, planner_query, goal_query, sink)
    }

    /// Curate one selector-issued review with independent cursor recovery.
    pub fn handle_selected_review<Q, O, S>(
        &self,
        selection: AgentSatisfactionReviewSelection,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        goal_query: &mut Q,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        O: AgentCommandOutcomeQuery,
        S: AgentGoalMutationSink,
    {
        let review = selection.review.clone();
        let mut report =
            AgentRuntimeReport::new(review.agent_id.clone(), selection.belief_revision_seq, 0);
        let selector = AgentSemanticSelector::new(self.store);
        if let Err(error) = selector.revalidate_satisfaction(&selection, belief_query) {
            push_storage_error(&mut report, error);
            return report;
        }
        match self
            .store
            .outcome_for_satisfaction_review(&selection.review)
        {
            Ok(Some(outcome)) => {
                report.delivered_count = 1;
                report.decision_count = 1;
                return self.finish_selected_review(
                    selection,
                    &outcome,
                    None,
                    outcome_query,
                    sink,
                    report,
                );
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        }
        let active_goals = match goal_query.active_goals_for_agent(&review.agent_id) {
            Ok(active_goals) => active_goals,
            Err(error) => {
                push_goal_query_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        let curation = AgentCuration::new(self.store);
        let input = match curation.assemble_satisfaction_input(
            &review,
            belief_query,
            planner_query,
            active_goals,
        ) {
            Ok(input) => input,
            Err(error) => {
                report.fatal_error(error.to_string());
                return report;
            }
        };
        let outcome = match curate_goal_satisfaction(input) {
            Ok(outcome) => outcome,
            Err(error) => {
                report.fatal_error(error.to_string());
                return report;
            }
        };
        if let Err(error) = selector.revalidate_satisfaction(&selection, belief_query) {
            push_storage_error(&mut report, error);
            return report;
        }
        let outcome = match self
            .store
            .put_selected_satisfaction_outcome(&selection, &outcome)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        };
        report.decision_count = 1;
        self.finish_selected_review(selection, &outcome, None, outcome_query, sink, report)
    }

    /// Complete one exact historical review without consulting the current belief head.
    pub fn handle_recovered_review<O, S>(
        &self,
        recovery: AgentSatisfactionRecoverySelection,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalMutationSink,
    {
        let mut report = AgentRuntimeReport::new(
            recovery.selection.review.agent_id.clone(),
            recovery.selection.belief_revision_seq,
            0,
        );
        let outcome = match self.store.satisfaction_recovery_outcome(&recovery) {
            Ok(outcome) => outcome,
            Err(error) => {
                push_storage_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        report.decision_count = 1;
        self.finish_selected_review(
            recovery.selection.clone(),
            &outcome,
            Some(&recovery),
            outcome_query,
            sink,
            report,
        )
    }

    fn finish_selected_review<O, S>(
        &self,
        selection: AgentSatisfactionReviewSelection,
        outcome: &AgentCurationOutcome,
        recovery: Option<&AgentSatisfactionRecoverySelection>,
        outcome_query: &mut O,
        sink: &mut S,
        mut report: AgentRuntimeReport,
    ) -> AgentRuntimeReport
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalMutationSink,
    {
        if !self.complete_selected_goal_mutation(
            outcome,
            recovery,
            outcome_query,
            sink,
            &mut report,
        ) {
            return report;
        }
        let advanced_at_seq = selection
            .expected_cursor_updated_at_seq
            .saturating_add(1)
            .max(selection.belief_revision_seq);
        match self
            .store
            .advance_satisfaction_cursor_cas(&AgentSatisfactionCursorCasIntent {
                selection,
                advanced_at_seq,
            }) {
            Ok(cursor) => report.output_sequence = cursor.last_reviewed_seq,
            Err(error) => push_storage_error(&mut report, error),
        }
        report
    }

    fn complete_selected_goal_mutation<O, S>(
        &self,
        outcome: &AgentCurationOutcome,
        recovery: Option<&AgentSatisfactionRecoverySelection>,
        outcome_query: &mut O,
        sink: &mut S,
        report: &mut AgentRuntimeReport,
    ) -> bool
    where
        O: AgentCommandOutcomeQuery,
        S: AgentGoalMutationSink,
    {
        match existing_sink_receipt(self.store, &outcome.decision) {
            Ok(Some(receipt)) => {
                report.sink_receipts.push(receipt);
                return true;
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(report, error);
                return false;
            }
        }
        let Some(command) = &outcome.goal_mutation_command else {
            if outcome.decision.decision == AgentDecisionKind::GoalMutationCommand {
                report.fatal_error("durable mutation decision has no exact command outbox");
                return false;
            }
            return true;
        };
        let authored = AgentAuthoredCommand::GoalMutation(Box::new(command.clone()));
        match outcome_query.committed_submission(&outcome.decision, &authored) {
            Ok(Some(submission)) => {
                if !self.revalidate_satisfaction_recovery(recovery, report) {
                    return false;
                }
                return record_sink_receipt(
                    self.store,
                    &outcome.decision,
                    AgentSinkReceiptKind::GoalMutationCommand,
                    submission,
                    &command.command_id,
                    report,
                );
            }
            Ok(None) => {}
            Err(error) => {
                push_outcome_query_error(report, error);
                return false;
            }
        }
        if !self.revalidate_satisfaction_recovery(recovery, report) {
            return false;
        }
        report.sink_submission_count += 1;
        match sink.submit_goal_mutation(command) {
            Ok(submission) => record_sink_receipt(
                self.store,
                &outcome.decision,
                AgentSinkReceiptKind::GoalMutationCommand,
                submission,
                &command.command_id,
                report,
            ),
            Err(error) => {
                push_sink_error(report, error);
                false
            }
        }
    }

    fn revalidate_satisfaction_recovery(
        &self,
        recovery: Option<&AgentSatisfactionRecoverySelection>,
        report: &mut AgentRuntimeReport,
    ) -> bool {
        let Some(recovery) = recovery else {
            return true;
        };
        match self.store.satisfaction_recovery_outcome(recovery) {
            Ok(_) => true,
            Err(error) => {
                push_storage_error(report, error);
                false
            }
        }
    }

    fn handle_review_core<Q, S>(
        &self,
        review: AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        goal_query: &mut Q,
        sink: &mut S,
    ) -> AgentRuntimeReport
    where
        Q: AgentActiveGoalQuery,
        S: AgentGoalMutationSink,
    {
        let mut report = AgentRuntimeReport::new(review.agent_id.clone(), review.review_seq, 0);
        if let Err(error) = review.validate() {
            report.fatal_error(error.to_string());
            return report;
        }

        let active_goals = match goal_query.active_goals_for_agent(&review.agent_id) {
            Ok(active_goals) => active_goals,
            Err(error) => {
                push_goal_query_error(&mut report, error);
                return report;
            }
        };
        report.delivered_count = 1;
        let Some(outcome) = self.persist_satisfaction_decision(
            review,
            belief_query,
            planner_query,
            active_goals.clone(),
            &mut report,
        ) else {
            return report;
        };

        if self.submit_goal_mutation_if_needed(&outcome, &active_goals, sink, &mut report) {
            report.output_sequence = report.input_sequence;
        }
        report
    }

    fn persist_satisfaction_decision(
        &self,
        review: AgentSatisfactionReview,
        belief_query: &BeliefQuery<'_>,
        planner_query: &PlannerQuery<'_>,
        active_goals: ActiveGoalSummary,
        report: &mut AgentRuntimeReport,
    ) -> Option<AgentCurationOutcome> {
        let curation = AgentCuration::new(self.store);
        let input = match curation.assemble_satisfaction_input(
            &review,
            belief_query,
            planner_query,
            active_goals,
        ) {
            Ok(input) => input,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        let outcome = match curate_goal_satisfaction(input) {
            Ok(outcome) => outcome,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        match self.store.decision_by_satisfaction_review(&review) {
            Ok(Some(existing)) => {
                report.decision_count = 1;
                return match self.store.decision_outbox(&existing.decision_id) {
                    Ok(Some(_)) => match self.store.outcome_for_satisfaction_review(&review) {
                        Ok(Some(outcome)) => Some(outcome),
                        Ok(None) => {
                            report.fatal_error("satisfaction decision disappeared during replay");
                            None
                        }
                        Err(error) => {
                            report.fatal_error(error.to_string());
                            None
                        }
                    },
                    Ok(None) => Some(AgentCurationOutcome {
                        decision: existing,
                        goal_command: None,
                        goal_mutation_command: None,
                    }),
                    Err(error) => {
                        report.fatal_error(error.to_string());
                        None
                    }
                };
            }
            Ok(None) => {}
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        }
        let persisted = match self
            .store
            .put_satisfaction_decision(&review, &outcome.decision)
        {
            Ok(persisted) => persisted,
            Err(error) => {
                report.fatal_error(error.to_string());
                return None;
            }
        };
        report.decision_count = 1;
        if persisted == outcome.decision {
            Some(outcome)
        } else {
            Some(AgentCurationOutcome {
                decision: persisted,
                goal_command: None,
                goal_mutation_command: None,
            })
        }
    }

    fn submit_goal_mutation_if_needed<S>(
        &self,
        outcome: &AgentCurationOutcome,
        _active_goals: &ActiveGoalSummary,
        sink: &mut S,
        report: &mut AgentRuntimeReport,
    ) -> bool
    where
        S: AgentGoalMutationSink,
    {
        match existing_sink_receipt(self.store, &outcome.decision) {
            Ok(Some(receipt)) => {
                report.sink_receipts.push(receipt);
                return true;
            }
            Ok(None) => {}
            Err(error) => {
                push_storage_error(report, error);
                return false;
            }
        }

        if outcome.decision.decision == AgentDecisionKind::GoalMutationCommand {
            report.fatal_error(
                "compatibility satisfaction curation cannot emit commands without a selector-issued owner fence",
            );
            return false;
        }
        let Some(command) = &outcome.goal_mutation_command else {
            return true;
        };

        report.sink_submission_count += 1;
        match sink.submit_goal_mutation(command) {
            Ok(submission) => record_sink_receipt(
                self.store,
                &outcome.decision,
                AgentSinkReceiptKind::GoalMutationCommand,
                submission,
                &command.command_id,
                report,
            ),
            Err(error) => {
                push_sink_error(report, error);
                false
            }
        }
    }
}

fn existing_sink_receipt(
    store: &AgentStore,
    decision: &crate::agent::contracts::AgentCurationDecision,
) -> Result<Option<AgentSinkReceipt>, StorageError> {
    store.sink_receipt_by_decision(&decision.decision_id)
}

fn record_sink_receipt(
    store: &AgentStore,
    decision: &crate::agent::contracts::AgentCurationDecision,
    kind: AgentSinkReceiptKind,
    submission: AgentSinkSubmission,
    expected_command_id: &str,
    report: &mut AgentRuntimeReport,
) -> bool {
    if submission.command_id != expected_command_id {
        report.fatal_error("sink submission command id mismatch");
        return false;
    }
    if let Err(error) = submission.validate() {
        report.fatal_error(error.to_string());
        return false;
    }
    let receipt = AgentSinkReceipt::new(decision, kind, submission);
    match store.put_sink_receipt(&receipt) {
        Ok(receipt) => {
            if let Err(error) = store.flush() {
                report.fatal_error(error.to_string());
                false
            } else {
                report.sink_receipts.push(receipt);
                true
            }
        }
        Err(error) => {
            report.fatal_error(error.to_string());
            false
        }
    }
}

fn push_sink_error(report: &mut AgentRuntimeReport, error: AgentSinkError) {
    if error.retryable {
        report.retryable_error(error.message);
    } else {
        report.fatal_error(error.message);
    }
}

fn push_goal_query_error(report: &mut AgentRuntimeReport, error: AgentActiveGoalQueryError) {
    if error.retryable {
        report.retryable_error(error.message);
    } else {
        report.fatal_error(error.message);
    }
}

fn push_outcome_query_error(report: &mut AgentRuntimeReport, error: AgentCommandOutcomeQueryError) {
    if error.retryable {
        report.retryable_error(error.message);
    } else {
        report.fatal_error(error.message);
    }
}

fn push_storage_error(report: &mut AgentRuntimeReport, error: StorageError) {
    match error {
        StorageError::Backpressure(_)
        | StorageError::Unavailable(_)
        | StorageError::DurabilityIndeterminate(_)
        | StorageError::IoError(_) => report.retryable_error(error.to_string()),
        StorageError::InvalidPath(_)
        | StorageError::MigrationConflict(_)
        | StorageError::IdentityMismatch { .. }
        | StorageError::RetentionGap { .. } => report.fatal_error(error.to_string()),
    }
}
