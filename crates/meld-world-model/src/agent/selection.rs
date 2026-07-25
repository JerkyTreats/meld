//! Bounded eligibility selection from durable agent state.
//!
//! Owner: world model agent domain. Goal-delivery and satisfaction-trigger
//! eligibility derive from durable subscription cursors, belief revision
//! heads, persisted decisions, sink receipts, and the satisfaction claim
//! checkpoint — never from caller-manufactured deliveries or review
//! sequences. Selection order is deterministic (subscription creation order,
//! the same order the store lists them in) and bounded by the caller budget,
//! so repeated selection over unchanged durable state returns nothing.

use crate::agent::contracts::{
    AgentDecisionKind, AgentDelivery, AgentSatisfactionCheckpoint, AgentSatisfactionReview,
};
use crate::agent::store::AgentStore;
use crate::belief::BeliefQuery;
use crate::error::StorageError;

/// Bounded set of goal deliveries eligible for curation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDeliverySelection {
    /// Eligible deliveries in deterministic subscription order.
    pub items: Vec<AgentDelivery>,
    /// True when eligible work remained beyond the budget.
    pub more_available: bool,
}

/// One satisfaction review trigger derived from durable state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSatisfactionTrigger {
    /// Claimed review identity, stable across crash replay.
    pub review: AgentSatisfactionReview,
    /// Belief revision the review inspects.
    pub belief_revision_id: String,
}

/// Bounded set of satisfaction triggers eligible for review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSatisfactionSelection {
    /// Eligible triggers in deterministic subscription order.
    pub items: Vec<AgentSatisfactionTrigger>,
    /// True when eligible work remained beyond the budget.
    pub more_available: bool,
}

/// Completion state of the trigger a checkpoint claims.
enum TriggerState {
    /// No decision was persisted for the claimed review.
    NotReviewed,
    /// A mutation decision exists but its sink receipt is missing.
    PendingReceipt,
    /// The claimed review reached a durable terminal state.
    Complete,
}

/// Derives bounded agent work from durable state and exact-key belief reads.
pub struct AgentWorkSelector<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentWorkSelector<'a> {
    /// Create a selector over durable agent storage.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Select goal deliveries whose belief stream moved past the cursor.
    ///
    /// A subscription is eligible when the current revision of its exact
    /// belief key sits beyond the durable delivery cursor. Each subscription
    /// contributes at most its current head per step, so selection is
    /// bounded by the subscription count and the caller budget.
    pub fn select_deliveries(
        &self,
        agent_id: &str,
        belief_query: &BeliefQuery<'_>,
        max_items: usize,
    ) -> Result<AgentDeliverySelection, StorageError> {
        let mut items = Vec::new();
        let mut more_available = false;
        for subscription in self.store.pending_subscriptions(agent_id)? {
            let Some(revision) = belief_query.current_revision(&subscription.belief_key)? else {
                continue;
            };
            if revision.source_cursor_end <= subscription.last_delivered_seq {
                continue;
            }
            if items.len() == max_items {
                more_available = true;
                break;
            }
            items.push(AgentDelivery {
                agent_id: subscription.agent_id,
                subscription_id: subscription.subscription_id,
                belief_revision_id: revision.revision_id,
                revision_seq: revision.source_cursor_end,
            });
        }
        Ok(AgentDeliverySelection {
            items,
            more_available,
        })
    }

    /// Select satisfaction triggers and claim review identity for new revisions.
    ///
    /// Eligibility rules, all from durable state:
    /// - a new revision on a subscribed key claims a fresh review at
    ///   `claim_seq` (durable before the trigger is returned);
    /// - an unchanged revision whose claimed review was absorbed or
    ///   indeterminate is ineligible — one absorbed review closes the
    ///   window until the next revision;
    /// - a claimed review with a persisted mutation decision but no sink
    ///   receipt replays with the same review sequence, and blocks claiming
    ///   a newer revision until its receipt lands.
    pub fn select_satisfaction_triggers(
        &self,
        agent_id: &str,
        belief_query: &BeliefQuery<'_>,
        claim_seq: u64,
        max_items: usize,
    ) -> Result<AgentSatisfactionSelection, StorageError> {
        let mut items = Vec::new();
        let mut more_available = false;
        for subscription in self.store.pending_subscriptions(agent_id)? {
            let Some(revision) = belief_query.current_revision(&subscription.belief_key)? else {
                continue;
            };
            let checkpoint = self
                .store
                .get_satisfaction_checkpoint(agent_id, &subscription.subscription_id)?;
            let trigger = match checkpoint {
                Some(checkpoint) if checkpoint.belief_revision_id == revision.revision_id => {
                    match self.trigger_state(&checkpoint)? {
                        TriggerState::NotReviewed | TriggerState::PendingReceipt => {
                            Some(trigger_from(&checkpoint))
                        }
                        TriggerState::Complete => None,
                    }
                }
                Some(checkpoint) => match self.trigger_state(&checkpoint)? {
                    // A pending mutation must resolve to a receipt before the
                    // claim advances, or its replayable review would be lost.
                    TriggerState::PendingReceipt => Some(trigger_from(&checkpoint)),
                    TriggerState::NotReviewed | TriggerState::Complete => Some(self.claim(
                        agent_id,
                        &subscription.subscription_id,
                        &revision.revision_id,
                        claim_seq,
                        Some(&checkpoint),
                    )?),
                },
                None => Some(self.claim(
                    agent_id,
                    &subscription.subscription_id,
                    &revision.revision_id,
                    claim_seq,
                    None,
                )?),
            };
            let Some(trigger) = trigger else {
                continue;
            };
            if items.len() == max_items {
                more_available = true;
                break;
            }
            items.push(trigger);
        }
        Ok(AgentSatisfactionSelection {
            items,
            more_available,
        })
    }

    /// Derive the completion state of a claimed review from durable records.
    fn trigger_state(
        &self,
        checkpoint: &AgentSatisfactionCheckpoint,
    ) -> Result<TriggerState, StorageError> {
        let Some(decision) = self
            .store
            .decision_by_satisfaction_review(&checkpoint.review())?
        else {
            return Ok(TriggerState::NotReviewed);
        };
        if decision.decision != AgentDecisionKind::GoalMutationCommand {
            // Absorbed and indeterminate reviews are terminal for their
            // revision: quiescence until the belief stream moves again.
            return Ok(TriggerState::Complete);
        }
        if self
            .store
            .sink_receipt_by_decision(&decision.decision_id)?
            .is_some()
        {
            return Ok(TriggerState::Complete);
        }
        Ok(TriggerState::PendingReceipt)
    }

    /// Durably claim a review sequence for one revision and subscription.
    fn claim(
        &self,
        agent_id: &str,
        subscription_id: &str,
        revision_id: &str,
        claim_seq: u64,
        previous: Option<&AgentSatisfactionCheckpoint>,
    ) -> Result<AgentSatisfactionTrigger, StorageError> {
        if let Some(previous) = previous {
            // Review identity must move strictly forward or the new claim
            // would collide with the previous revision's decision index.
            if claim_seq <= previous.review_seq {
                return Err(StorageError::Backpressure(format!(
                    "satisfaction claim sequence {claim_seq} does not advance past prior review {}",
                    previous.review_seq
                )));
            }
        }
        let checkpoint = AgentSatisfactionCheckpoint {
            agent_id: agent_id.to_string(),
            subscription_id: subscription_id.to_string(),
            belief_revision_id: revision_id.to_string(),
            review_seq: claim_seq,
            claimed_at_seq: claim_seq,
        };
        self.store.put_satisfaction_checkpoint(&checkpoint)?;
        // The claim must be durable before the review runs so crash replay
        // reuses this review identity instead of inventing a new sequence.
        self.store.flush()?;
        Ok(trigger_from(&checkpoint))
    }
}

fn trigger_from(checkpoint: &AgentSatisfactionCheckpoint) -> AgentSatisfactionTrigger {
    AgentSatisfactionTrigger {
        review: checkpoint.review(),
        belief_revision_id: checkpoint.belief_revision_id.clone(),
    }
}
