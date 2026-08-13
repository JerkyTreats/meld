//! Bounded goal curation and satisfaction actors.
//!
//! Owner: world model agent domain. One step processes at most a budgeted
//! number of eligible items under an injected sequence — no wall clock and
//! no caller-manufactured deliveries or review sequences. Eligibility comes
//! from durable state through [`crate::agent::selection`], the curation rule
//! is frozen by exact runtime composition with a record-backed compatibility
//! path, and curated output crosses into execution only through the named
//! [`CurationGoalSetPort`] seam. A step
//! over unchanged durable state selects nothing and commits no work.
//!
//! The request and report pair is domain-owned. Root runtime contracts adapt
//! this report behind their own bounded-step surface; this crate does not
//! depend on them.

use std::sync::Arc;

use crate::agent::goal_port::CurationGoalSetPort;
use crate::agent::runtime::{
    AgentActiveGoalQuery, AgentGoalCurationRuntime, AgentRuntimeReport,
    AgentSatisfactionCurationRuntime,
};
use crate::agent::selection::AgentWorkSelector;
use crate::agent::store::AgentStore;
use crate::agent::strategy::AgentStrategyRuntimeConfig;
use crate::agent::{AgentCurationRuleBinding, AgentSinkReceipt};
use crate::belief::{BeliefQuery, BeliefStore};
use crate::error::StorageError;
use crate::planner::PlannerQuery;
use crate::waiting::{conditions, WaitingOnDeclaration};
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::TraversalQuery;

/// Bounded agent actor step request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStepRequest {
    /// Injected step sequence used to claim new satisfaction reviews.
    pub sequence: u64,
    /// Maximum eligible items to attempt during one step.
    pub max_items: usize,
}

/// Diagnostic issue produced by one agent actor step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStepIssue {
    /// Subscription id the issue concerns, when known.
    pub item_id: Option<String>,
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Domain report from one bounded agent actor step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStepReport {
    /// Stable actor identifier.
    pub actor_id: String,
    /// Injected sequence supplied by the request.
    pub input_sequence: u64,
    /// Eligible items selected from durable state this step.
    pub items_selected: usize,
    /// Selected items attempted this step.
    pub items_attempted: usize,
    /// Curation decisions durably persisted or reused this step.
    pub decisions_persisted: usize,
    /// Sink submissions attempted through the named port.
    pub sink_submissions: usize,
    /// Durable sink receipts observed or recorded this step.
    pub sink_receipts: Vec<AgentSinkReceipt>,
    /// Retryable diagnostics observed during the step.
    pub retryable_errors: Vec<AgentStepIssue>,
    /// Fatal diagnostics observed during the step.
    pub fatal_errors: Vec<AgentStepIssue>,
    /// True when eligible work remained after the budget was consumed.
    pub budget_exhausted: bool,
    /// What would make quiet curation work eligible (DBG-016).
    ///
    /// Derived from the selection this step already computed; emission
    /// never gates or reorders curation.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

impl AgentStepReport {
    fn new(actor_id: &str, input_sequence: u64) -> Self {
        Self {
            actor_id: actor_id.to_string(),
            input_sequence,
            items_selected: 0,
            items_attempted: 0,
            decisions_persisted: 0,
            sink_submissions: 0,
            sink_receipts: Vec::new(),
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        }
    }

    fn fatal(&mut self, item_id: Option<String>, code: &str, message: &str) {
        self.fatal_errors.push(issue(item_id, code, message));
    }

    fn retryable(&mut self, item_id: Option<String>, code: &str, message: &str) {
        self.retryable_errors.push(issue(item_id, code, message));
    }

    /// Fold one runtime tick report into the step report.
    fn absorb_tick(&mut self, item_id: &str, tick: AgentRuntimeReport) {
        self.decisions_persisted += tick.decision_count;
        self.sink_submissions += tick.sink_submission_count;
        self.sink_receipts.extend(tick.sink_receipts);
        for message in tick.retryable_errors {
            self.retryable(Some(item_id.to_string()), "item_retryable", &message);
        }
        for message in tick.fatal_errors {
            self.fatal(Some(item_id.to_string()), "item_failed", &message);
        }
    }

    /// Classify a selection error: contention retries, corruption is fatal.
    fn selection_error(&mut self, error: StorageError) {
        match error {
            StorageError::Backpressure(message) => {
                self.retryable(None, "selection_contended", &message)
            }
            error => self.fatal(None, "selection_failed", &error.to_string()),
        }
    }
}

/// Shared durable bindings for both bounded agent actors.
struct AgentActorCore {
    actor_id: String,
    agent_id: String,
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
}

impl AgentActorCore {
    /// Record a zero-budget fatal on the step report.
    ///
    /// Returns false when the budget is invalid and the step must stop.
    fn begin(&self, request: &AgentStepRequest, report: &mut AgentStepReport) -> bool {
        if request.max_items == 0 {
            report.fatal(
                None,
                "invalid_budget",
                "agent step budget must be greater than zero",
            );
            return false;
        }
        true
    }
}

/// Bounded actor that discovers and curates goal deliveries from durable state.
pub struct AgentGoalCurationActor {
    core: AgentActorCore,
    strategy: Option<AgentStrategyRuntimeConfig>,
    curation_rule: Option<AgentCurationRuleBinding>,
}

impl AgentGoalCurationActor {
    /// Bind the actor to durable stores and one agent identity.
    pub fn new(
        actor_id: impl Into<String>,
        agent_id: impl Into<String>,
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            core: AgentActorCore {
                actor_id: actor_id.into(),
                agent_id: agent_id.into(),
                agent_store,
                belief_store,
                traversal_store,
            },
            strategy: None,
            curation_rule: None,
        }
    }

    /// Bind the actor to stores and an activated minimal Strategy configuration.
    pub fn new_with_strategy(
        actor_id: impl Into<String>,
        agent_id: impl Into<String>,
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
        strategy: AgentStrategyRuntimeConfig,
    ) -> Self {
        Self {
            core: AgentActorCore {
                actor_id: actor_id.into(),
                agent_id: agent_id.into(),
                agent_store,
                belief_store,
                traversal_store,
            },
            strategy: Some(strategy),
            curation_rule: None,
        }
    }

    /// Freeze the exact curation-rule revision selected for this composition.
    pub fn with_curation_rule(mut self, curation_rule: AgentCurationRuleBinding) -> Self {
        self.curation_rule = Some(curation_rule);
        self
    }

    /// Stable actor identity carried in reports.
    pub fn actor_id(&self) -> &str {
        &self.core.actor_id
    }

    /// Run one bounded goal curation step at the injected sequence.
    ///
    /// Sequencing per item is owned by the curation runtime: decision
    /// persisted and flushed, then port submission, then receipt, then
    /// cursor advance. The exact composition rule takes precedence over the
    /// compatibility record body, and the step fails truthfully when neither
    /// is available.
    pub fn bounded_step<Q, P>(
        &self,
        request: &AgentStepRequest,
        goal_query: &mut Q,
        port: &mut P,
    ) -> AgentStepReport
    where
        Q: AgentActiveGoalQuery,
        P: CurationGoalSetPort,
    {
        let mut report = AgentStepReport::new(&self.core.actor_id, request.sequence);
        if !self.core.begin(request, &mut report) {
            return report;
        }
        let mut agent = match self.core.agent_store.get_agent(&self.core.agent_id) {
            Ok(Some(agent)) => agent,
            Ok(None) => {
                report.fatal(
                    None,
                    "unknown_agent",
                    &format!("unknown agent '{}'", self.core.agent_id),
                );
                return report;
            }
            Err(error) => {
                report.fatal(None, "agent_read_failed", &error.to_string());
                return report;
            }
        };
        if let Some(curation_rule) = &self.curation_rule {
            agent.curation_rule = Some(curation_rule.clone());
        }
        let rule = match agent.installed_curation_rule() {
            Ok(rule) => rule.clone(),
            Err(error) => {
                report.fatal(None, "curation_rule_missing", &error.to_string());
                return report;
            }
        };

        let belief_query = BeliefQuery::new(&self.core.belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&self.core.belief_store),
            TraversalQuery::new(&self.core.traversal_store),
        );
        let selection = match AgentWorkSelector::new(&self.core.agent_store).select_deliveries(
            &self.core.agent_id,
            &belief_query,
            request.max_items,
        ) {
            Ok(selection) => selection,
            Err(error) => {
                report.selection_error(error);
                return report;
            }
        };
        report.budget_exhausted = selection.more_available;
        report.items_selected = selection.items.len();
        // The hardened DBG-016 rule: a quiet selection states what would
        // change it — a revision newer than the delivery cursor.
        if selection.items.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: conditions::NO_UNDELIVERED_REVISIONS.to_string(),
                subject_key: Some(agent.subject.index_key()),
                detail: format!(
                    "every subscription of agent '{}' has consumed its latest revision",
                    self.core.agent_id
                ),
            });
        }

        let runtime = match &self.strategy {
            Some(strategy) => AgentGoalCurationRuntime::new_with_strategy(
                &self.core.agent_store,
                strategy.clone(),
            ),
            None => AgentGoalCurationRuntime::new(&self.core.agent_store),
        };
        let runtime = match self
            .curation_rule
            .as_ref()
            .and_then(|binding| binding.revision.clone())
        {
            Some(revision) => runtime.with_curation_rule_revision(revision),
            None => runtime,
        };
        for delivery in selection.items {
            report.items_attempted += 1;
            let item_id = delivery.subscription_id.clone();
            let tick = runtime.handle_delivery_with_goal_query(
                delivery,
                &belief_query,
                &planner_query,
                &mut *goal_query,
                rule.clone(),
                &mut *port,
            );
            report.absorb_tick(&item_id, tick);
        }
        report
    }
}

/// Bounded actor that reviews eligible satisfaction triggers from durable state.
pub struct AgentSatisfactionCurationActor {
    core: AgentActorCore,
}

impl AgentSatisfactionCurationActor {
    /// Bind the actor to durable stores and one agent identity.
    pub fn new(
        actor_id: impl Into<String>,
        agent_id: impl Into<String>,
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            core: AgentActorCore {
                actor_id: actor_id.into(),
                agent_id: agent_id.into(),
                agent_store,
                belief_store,
                traversal_store,
            },
        }
    }

    /// Stable actor identity carried in reports.
    pub fn actor_id(&self) -> &str {
        &self.core.actor_id
    }

    /// Run one bounded satisfaction review step at the injected sequence.
    ///
    /// New revisions claim their review identity durably before the review
    /// runs, so crash replay reuses the same review sequence. Per item the
    /// satisfaction runtime persists the decision before the port sees the
    /// mutation command.
    pub fn bounded_step<Q, P>(
        &self,
        request: &AgentStepRequest,
        goal_query: &mut Q,
        port: &mut P,
    ) -> AgentStepReport
    where
        Q: AgentActiveGoalQuery,
        P: CurationGoalSetPort,
    {
        let mut report = AgentStepReport::new(&self.core.actor_id, request.sequence);
        if !self.core.begin(request, &mut report) {
            return report;
        }
        let agent = match self.core.agent_store.get_agent(&self.core.agent_id) {
            Ok(Some(agent)) => agent,
            Ok(None) => {
                report.fatal(
                    None,
                    "unknown_agent",
                    &format!("unknown agent '{}'", self.core.agent_id),
                );
                return report;
            }
            Err(error) => {
                report.fatal(None, "agent_read_failed", &error.to_string());
                return report;
            }
        };

        let belief_query = BeliefQuery::new(&self.core.belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&self.core.belief_store),
            TraversalQuery::new(&self.core.traversal_store),
        );
        let selection = match AgentWorkSelector::new(&self.core.agent_store)
            .select_satisfaction_triggers(
                &self.core.agent_id,
                &belief_query,
                request.sequence,
                request.max_items,
            ) {
            Ok(selection) => selection,
            Err(error) => {
                report.selection_error(error);
                return report;
            }
        };
        report.budget_exhausted = selection.more_available;
        report.items_selected = selection.items.len();
        // The hardened DBG-016 rule: no pending review means satisfaction
        // waits on a newer revision or an outstanding sink receipt.
        if selection.items.is_empty() {
            report.waiting_on.push(WaitingOnDeclaration {
                condition: conditions::NO_PENDING_SATISFACTION_REVIEWS.to_string(),
                subject_key: Some(agent.subject.index_key()),
                detail: format!(
                    "no unreviewed revision or receipted decision awaits review for agent '{}'",
                    self.core.agent_id
                ),
            });
        }

        let runtime = AgentSatisfactionCurationRuntime::new(&self.core.agent_store);
        for trigger in selection.items {
            report.items_attempted += 1;
            let item_id = trigger.review.subscription_id.clone();
            let tick = runtime.handle_review_with_goal_query(
                trigger.review,
                &belief_query,
                &planner_query,
                &mut *goal_query,
                &mut *port,
            );
            report.absorb_tick(&item_id, tick);
        }
        report
    }
}

fn issue(item_id: Option<String>, code: &str, message: &str) -> AgentStepIssue {
    AgentStepIssue {
        item_id,
        code: code.to_string(),
        message: message.to_string(),
    }
}
