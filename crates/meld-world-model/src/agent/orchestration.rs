//! Bounded recurring agent actors over durable semantic selectors.

use std::sync::Arc;

use crate::agent::{
    AgentActiveGoalQuery, AgentCommandOutcomeQuery, AgentGoalCommandSink, AgentGoalCurationRuntime,
    AgentGoalMutationSink, AgentRuntimeReport, AgentSatisfactionCurationRuntime,
    AgentSelectedGoalTick, AgentSemanticSelector, AgentStore, BELIEF_REVISION_REVIEW_SOURCE,
};
use crate::belief::{BeliefQuery, BeliefStore};
use crate::error::StorageError;
use crate::planner::PlannerQuery;
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::TraversalQuery;

/// Canonical recurring goal-curation actor identity.
pub const AGENT_GOAL_CURATION_ACTOR_ID: &str = "world_model.agent_goal_curation";

/// Canonical recurring satisfaction-curation actor identity.
pub const AGENT_SATISFACTION_CURATION_ACTOR_ID: &str = "world_model.satisfaction_curation";

/// Maximum selected agent inputs for one bounded curation tick.
pub const MAX_AGENT_CURATION_ITEMS: usize = 1024;

/// Request shared by recurring agent curation actors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCurationTickRequest {
    /// Maximum pending semantic inputs selected in durable order.
    pub max_items: usize,
}

/// Stable issue emitted by one bounded agent actor pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCurationActorIssue {
    /// Agent or subscription identity associated with the issue.
    pub item_id: Option<String>,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic detail.
    pub message: String,
}

/// Aggregated report for one domain-owned agent actor pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCurationActorReport {
    /// Canonical recurring actor identity.
    pub actor_id: String,
    /// Highest selected belief input sequence.
    pub input_sequence: u64,
    /// Highest durable cursor sequence advanced by this pass.
    pub output_sequence: u64,
    /// Pending deliveries or reviews selected for work.
    pub selected_count: usize,
    /// Durable decisions persisted or recovered.
    pub decision_count: usize,
    /// Durable execution sink receipts observed or recorded.
    pub sink_receipt_count: usize,
    /// Semantic cursors advanced by the pass.
    pub cursor_advanced_count: usize,
    /// Retryable issues that preserve pending durable input.
    pub retryable_errors: Vec<AgentCurationActorIssue>,
    /// Fatal issues that require data or configuration repair.
    pub fatal_errors: Vec<AgentCurationActorIssue>,
    /// True when the durable candidate scan reached its bounded budget.
    pub budget_exhausted: bool,
}

impl AgentCurationActorReport {
    fn empty(actor_id: &str) -> Self {
        Self {
            actor_id: actor_id.to_string(),
            input_sequence: 0,
            output_sequence: 0,
            selected_count: 0,
            decision_count: 0,
            sink_receipt_count: 0,
            cursor_advanced_count: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }

    fn absorb(&mut self, item_id: String, report: AgentRuntimeReport) {
        self.decision_count += report.decision_count;
        self.sink_receipt_count += report.sink_receipts.len();
        self.input_sequence = self.input_sequence.max(report.input_sequence);
        self.output_sequence = self.output_sequence.max(report.output_sequence);
        if report.output_sequence >= report.input_sequence && report.output_sequence > 0 {
            self.cursor_advanced_count += 1;
        }
        self.retryable_errors
            .extend(
                report
                    .retryable_errors
                    .into_iter()
                    .map(|message| AgentCurationActorIssue {
                        item_id: Some(item_id.clone()),
                        code: "agent_curation_retryable".to_string(),
                        message,
                    }),
            );
        self.fatal_errors
            .extend(
                report
                    .fatal_errors
                    .into_iter()
                    .map(|message| AgentCurationActorIssue {
                        item_id: Some(item_id.clone()),
                        code: "agent_curation_fatal".to_string(),
                        message,
                    }),
            );
    }
}

/// Agent-owned bounded actor for pending belief deliveries.
pub struct AgentGoalCurationActor {
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
}

impl AgentGoalCurationActor {
    /// Bind durable agent state and public world-model query authorities.
    pub fn new(
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            agent_store,
            belief_store,
            traversal_store,
        }
    }

    /// Select and curate one bounded delivery window in domain-owned order.
    pub fn tick<Q, O, S>(
        &self,
        request: AgentCurationTickRequest,
        goal_query: &mut Q,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentCurationActorReport
    where
        Q: AgentActiveGoalQuery,
        O: AgentCommandOutcomeQuery,
        S: AgentGoalCommandSink,
    {
        let mut report = AgentCurationActorReport::empty(AGENT_GOAL_CURATION_ACTOR_ID);
        if !valid_budget(request.max_items) {
            push_invalid_budget(&mut report);
            return report;
        }
        let belief_query = BeliefQuery::new(self.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(self.belief_store.as_ref()),
            TraversalQuery::new(self.traversal_store.as_ref()),
        );
        let selector = AgentSemanticSelector::new(self.agent_store.as_ref());
        let recoveries = match selector.select_goal_recoveries_bounded(request.max_items) {
            Ok(recoveries) => recoveries,
            Err(error) => {
                push_storage(&mut report, None, error);
                return report;
            }
        };
        let runtime = AgentGoalCurationRuntime::new(self.agent_store.as_ref());
        if !recoveries.items.is_empty() {
            report.budget_exhausted = recoveries.budget_exhausted;
            report.selected_count = recoveries.items.len();
            for recovery in recoveries.items {
                let item_id = recovery.selection.delivery.subscription_id.clone();
                report.input_sequence = report
                    .input_sequence
                    .max(recovery.selection.delivery.revision_seq);
                let item_report = runtime.handle_recovered_delivery(recovery, outcome_query, sink);
                report.absorb(item_id, item_report);
            }
            return report;
        }
        let selections = match selector.select_deliveries_bounded(&belief_query, request.max_items)
        {
            Ok(selections) => selections,
            Err(error) => {
                push_storage(&mut report, None, error);
                return report;
            }
        };
        report.budget_exhausted = selections.budget_exhausted;
        report.selected_count = selections.items.len();
        for selection in selections.items {
            let item_id = selection.delivery.subscription_id.clone();
            report.input_sequence = report.input_sequence.max(selection.delivery.revision_seq);
            let receipt = match self
                .agent_store
                .bootstrap_receipt_for_agent(&selection.delivery.agent_id)
            {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    report.fatal_errors.push(issue(
                        Some(item_id),
                        "agent_bootstrap_receipt_missing",
                        "selected agent has no completed bootstrap receipt".to_string(),
                    ));
                    continue;
                }
                Err(error) => {
                    push_storage(&mut report, Some(item_id), error);
                    continue;
                }
            };
            let rule = match self.agent_store.get_curation_rule(&receipt.rule_id) {
                Ok(Some(rule)) if rule.agent_id == selection.delivery.agent_id => rule,
                Ok(Some(_)) => {
                    report.fatal_errors.push(issue(
                        Some(item_id),
                        "agent_curation_rule_owner_mismatch",
                        "durable curation rule belongs to another agent".to_string(),
                    ));
                    continue;
                }
                Ok(None) => {
                    report.fatal_errors.push(issue(
                        Some(item_id),
                        "agent_curation_rule_missing",
                        "selected agent bootstrap references a missing curation rule".to_string(),
                    ));
                    continue;
                }
                Err(error) => {
                    push_storage(&mut report, Some(item_id), error);
                    continue;
                }
            };
            let item_report = runtime.handle_selected_delivery(
                AgentSelectedGoalTick {
                    selection,
                    belief_query: &belief_query,
                    planner_query: &planner_query,
                    rule_config: rule.config,
                },
                goal_query,
                outcome_query,
                sink,
            );
            report.absorb(item_id, item_report);
        }
        report
    }
}

/// Agent-owned bounded actor for pending satisfaction reviews.
pub struct AgentSatisfactionCurationActor {
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
}

impl AgentSatisfactionCurationActor {
    /// Bind durable agent state and public world-model query authorities.
    pub fn new(
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            agent_store,
            belief_store,
            traversal_store,
        }
    }

    /// Select and curate one bounded satisfaction window in domain-owned order.
    pub fn tick<Q, O, S>(
        &self,
        request: AgentCurationTickRequest,
        goal_query: &mut Q,
        outcome_query: &mut O,
        sink: &mut S,
    ) -> AgentCurationActorReport
    where
        Q: AgentActiveGoalQuery,
        O: AgentCommandOutcomeQuery,
        S: AgentGoalMutationSink,
    {
        let mut report = AgentCurationActorReport::empty(AGENT_SATISFACTION_CURATION_ACTOR_ID);
        if !valid_budget(request.max_items) {
            push_invalid_budget(&mut report);
            return report;
        }
        let belief_query = BeliefQuery::new(self.belief_store.as_ref());
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(self.belief_store.as_ref()),
            TraversalQuery::new(self.traversal_store.as_ref()),
        );
        let selector = AgentSemanticSelector::new(self.agent_store.as_ref());
        let recoveries = match selector.select_satisfaction_recoveries_bounded(request.max_items) {
            Ok(recoveries) => recoveries,
            Err(error) => {
                push_storage(&mut report, None, error);
                return report;
            }
        };
        let runtime = AgentSatisfactionCurationRuntime::new(self.agent_store.as_ref());
        if !recoveries.items.is_empty() {
            report.budget_exhausted = recoveries.budget_exhausted;
            report.selected_count = recoveries.items.len();
            for recovery in recoveries.items {
                let item_id = recovery.selection.review.subscription_id.clone();
                report.input_sequence = report
                    .input_sequence
                    .max(recovery.selection.belief_revision_seq);
                let item_report = runtime.handle_recovered_review(recovery, outcome_query, sink);
                report.absorb(item_id, item_report);
            }
            return report;
        }
        let selections = match selector.select_satisfaction_reviews_bounded(
            &belief_query,
            BELIEF_REVISION_REVIEW_SOURCE,
            request.max_items,
        ) {
            Ok(selections) => selections,
            Err(error) => {
                push_storage(&mut report, None, error);
                return report;
            }
        };
        report.budget_exhausted = selections.budget_exhausted;
        report.selected_count = selections.items.len();
        for selection in selections.items {
            let item_id = selection.review.subscription_id.clone();
            report.input_sequence = report.input_sequence.max(selection.belief_revision_seq);
            let item_report = runtime.handle_selected_review(
                selection,
                &belief_query,
                &planner_query,
                goal_query,
                outcome_query,
                sink,
            );
            report.absorb(item_id, item_report);
        }
        report
    }
}

fn valid_budget(max_items: usize) -> bool {
    max_items > 0 && max_items <= MAX_AGENT_CURATION_ITEMS
}

fn push_invalid_budget(report: &mut AgentCurationActorReport) {
    report.fatal_errors.push(issue(
        None,
        "invalid_request",
        format!("agent curation budget must be within 1..={MAX_AGENT_CURATION_ITEMS}"),
    ));
}

fn push_storage(
    report: &mut AgentCurationActorReport,
    item_id: Option<String>,
    error: StorageError,
) {
    let target = if matches!(
        error,
        StorageError::Backpressure(_)
            | StorageError::Unavailable(_)
            | StorageError::DurabilityIndeterminate(_)
            | StorageError::IoError(_)
    ) {
        &mut report.retryable_errors
    } else {
        &mut report.fatal_errors
    };
    target.push(issue(item_id, "agent_curation_storage", error.to_string()));
}

fn issue(item_id: Option<String>, code: &str, message: String) -> AgentCurationActorIssue {
    AgentCurationActorIssue {
        item_id,
        code: code.to_string(),
        message,
    }
}
