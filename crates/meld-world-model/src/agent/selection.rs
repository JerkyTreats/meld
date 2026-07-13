//! Deterministic agent-owned selection over durable belief and cursor state.
//!
//! Root assembly supplies stores and budgets only. This module decides which
//! operational agent input is eligible for curation and retains every record
//! fence that must be checked again before a semantic commit.

use crate::agent::contracts::{
    AgentDelivery, AgentDeliverySelection, AgentSatisfactionCursorIdentity,
    AgentSatisfactionReview, AgentSatisfactionReviewCursor, AgentSatisfactionReviewSelection,
    AgentStatus, AgentSubscriptionRecord, AgentSubscriptionStatus, BELIEF_REVISION_REVIEW_SOURCE,
};
use crate::agent::store::AgentStore;
use crate::belief::{BeliefKey, BeliefQuery, BeliefRevision};
use crate::error::StorageError;

/// Agent-domain selector for bounded belief delivery and satisfaction work.
pub struct AgentSemanticSelector<'a> {
    store: &'a AgentStore,
}

impl<'a> AgentSemanticSelector<'a> {
    /// Bind semantic selection to durable agent state.
    pub fn new(store: &'a AgentStore) -> Self {
        Self { store }
    }

    /// Select pending belief deliveries in one stable global order.
    pub fn select_deliveries(
        &self,
        belief_query: &BeliefQuery<'_>,
        limit: usize,
    ) -> Result<Vec<AgentDeliverySelection>, StorageError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut selections = Vec::new();
        for agent in self.store.agents_by_status(AgentStatus::Operational)? {
            for subscription in self.store.subscriptions_for_agent(&agent.agent_id)? {
                if subscription.status != AgentSubscriptionStatus::Active {
                    continue;
                }
                let Some(revision) = current_revision(belief_query, &subscription.belief_key)?
                else {
                    continue;
                };
                if !pending_revision(
                    "subscription delivery",
                    subscription.last_delivered_revision_id.as_deref(),
                    subscription.last_delivered_seq,
                    &revision,
                )? {
                    continue;
                }
                selections.push(AgentDeliverySelection {
                    delivery: AgentDelivery {
                        agent_id: agent.agent_id.clone(),
                        subscription_id: subscription.subscription_id.clone(),
                        belief_revision_id: revision.revision_id.clone(),
                        revision_seq: revision.source_cursor_end,
                    },
                    belief_key: subscription.belief_key.clone(),
                    expected_agent_updated_at_seq: agent.updated_at_seq,
                    expected_subscription_updated_at_seq: subscription.updated_at_seq,
                    expected_delivered_revision_id: subscription.last_delivered_revision_id.clone(),
                    expected_delivered_seq: subscription.last_delivered_seq,
                });
            }
        }
        selections.sort_by(|left, right| {
            left.delivery
                .revision_seq
                .cmp(&right.delivery.revision_seq)
                .then_with(|| left.delivery.agent_id.cmp(&right.delivery.agent_id))
                .then_with(|| {
                    left.delivery
                        .subscription_id
                        .cmp(&right.delivery.subscription_id)
                })
                .then_with(|| {
                    left.delivery
                        .belief_revision_id
                        .cmp(&right.delivery.belief_revision_id)
                })
        });
        selections.truncate(limit);
        Ok(selections)
    }

    /// Select pending satisfaction reviews from an independent durable cursor.
    pub fn select_satisfaction_reviews(
        &self,
        belief_query: &BeliefQuery<'_>,
        review_source: &str,
        limit: usize,
    ) -> Result<Vec<AgentSatisfactionReviewSelection>, StorageError> {
        if review_source != BELIEF_REVISION_REVIEW_SOURCE {
            return Err(StorageError::InvalidPath(format!(
                "unsupported satisfaction review source '{review_source}'"
            )));
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut selections = Vec::new();
        for agent in self.store.agents_by_status(AgentStatus::Operational)? {
            for subscription in self.store.subscriptions_for_agent(&agent.agent_id)? {
                if subscription.status != AgentSubscriptionStatus::Active {
                    continue;
                }
                let Some(revision) = current_revision(belief_query, &subscription.belief_key)?
                else {
                    continue;
                };
                let cursor_identity = AgentSatisfactionCursorIdentity {
                    agent_id: agent.agent_id.clone(),
                    subscription_id: subscription.subscription_id.clone(),
                    branch_id: subscription.belief_key.branch_scope.branch_id.clone(),
                    review_source: review_source.to_string(),
                };
                let cursor = self
                    .store
                    .satisfaction_review_cursor(&cursor_identity)?
                    .unwrap_or_else(|| {
                        AgentSatisfactionReviewCursor::empty(cursor_identity.clone())
                    });
                if !pending_revision(
                    "satisfaction review",
                    cursor.last_reviewed_revision_id.as_deref(),
                    cursor.last_reviewed_seq,
                    &revision,
                )? {
                    continue;
                }
                selections.push(AgentSatisfactionReviewSelection {
                    review: AgentSatisfactionReview {
                        agent_id: agent.agent_id.clone(),
                        subscription_id: subscription.subscription_id.clone(),
                        review_seq: revision.source_cursor_end,
                    },
                    belief_key: subscription.belief_key.clone(),
                    belief_revision_id: revision.revision_id.clone(),
                    belief_revision_seq: revision.source_cursor_end,
                    cursor_identity,
                    expected_agent_updated_at_seq: agent.updated_at_seq,
                    expected_subscription_updated_at_seq: subscription.updated_at_seq,
                    expected_reviewed_revision_id: cursor.last_reviewed_revision_id,
                    expected_reviewed_seq: cursor.last_reviewed_seq,
                    expected_cursor_updated_at_seq: cursor.updated_at_seq,
                });
            }
        }
        selections.sort_by(|left, right| {
            left.belief_revision_seq
                .cmp(&right.belief_revision_seq)
                .then_with(|| left.review.agent_id.cmp(&right.review.agent_id))
                .then_with(|| {
                    left.review
                        .subscription_id
                        .cmp(&right.review.subscription_id)
                })
                .then_with(|| left.belief_revision_id.cmp(&right.belief_revision_id))
        });
        selections.truncate(limit);
        Ok(selections)
    }

    /// Revalidate one delivery against current owner records before commit.
    pub fn revalidate_delivery(
        &self,
        selection: &AgentDeliverySelection,
        belief_query: &BeliefQuery<'_>,
    ) -> Result<(), StorageError> {
        selection.validate()?;
        let subscription = self.validate_agent_and_subscription(
            &selection.delivery.agent_id,
            &selection.delivery.subscription_id,
            &selection.belief_key,
            selection.expected_agent_updated_at_seq,
            selection.expected_subscription_updated_at_seq,
        )?;
        if subscription.last_delivered_revision_id != selection.expected_delivered_revision_id
            || subscription.last_delivered_seq != selection.expected_delivered_seq
        {
            return Err(stale("subscription delivery cursor changed"));
        }
        require_selected_revision(
            belief_query,
            &selection.belief_key,
            &selection.delivery.belief_revision_id,
            selection.delivery.revision_seq,
        )
    }

    /// Revalidate one satisfaction review against current owner records.
    pub fn revalidate_satisfaction(
        &self,
        selection: &AgentSatisfactionReviewSelection,
        belief_query: &BeliefQuery<'_>,
    ) -> Result<(), StorageError> {
        selection.validate()?;
        self.validate_agent_and_subscription(
            &selection.review.agent_id,
            &selection.review.subscription_id,
            &selection.belief_key,
            selection.expected_agent_updated_at_seq,
            selection.expected_subscription_updated_at_seq,
        )?;
        let cursor = self
            .store
            .satisfaction_review_cursor(&selection.cursor_identity)?
            .unwrap_or_else(|| {
                AgentSatisfactionReviewCursor::empty(selection.cursor_identity.clone())
            });
        if cursor.last_reviewed_revision_id != selection.expected_reviewed_revision_id
            || cursor.last_reviewed_seq != selection.expected_reviewed_seq
            || cursor.updated_at_seq != selection.expected_cursor_updated_at_seq
        {
            return Err(stale("satisfaction review cursor changed"));
        }
        require_selected_revision(
            belief_query,
            &selection.belief_key,
            &selection.belief_revision_id,
            selection.belief_revision_seq,
        )
    }

    fn validate_agent_and_subscription(
        &self,
        agent_id: &str,
        subscription_id: &str,
        belief_key: &BeliefKey,
        expected_agent_updated_at_seq: u64,
        expected_subscription_updated_at_seq: u64,
    ) -> Result<AgentSubscriptionRecord, StorageError> {
        let agent = self
            .store
            .get_agent(agent_id)?
            .ok_or_else(|| stale("selected agent is missing"))?;
        if agent.status != AgentStatus::Operational
            || agent.updated_at_seq != expected_agent_updated_at_seq
        {
            return Err(stale("selected agent lifecycle changed"));
        }
        let subscription = self
            .store
            .get_subscription(subscription_id)?
            .ok_or_else(|| stale("selected subscription is missing"))?;
        if subscription.agent_id != agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || subscription.belief_key != *belief_key
            || subscription.updated_at_seq != expected_subscription_updated_at_seq
        {
            return Err(stale("selected subscription changed"));
        }
        Ok(subscription)
    }
}

fn current_revision(
    belief_query: &BeliefQuery<'_>,
    belief_key: &BeliefKey,
) -> Result<Option<BeliefRevision>, StorageError> {
    let Some(view) = belief_query.current_view(belief_key)? else {
        return Ok(None);
    };
    let Some(revision_id) = view.current_revision_id else {
        return Ok(None);
    };
    let revision = belief_query
        .revision_history(belief_key)?
        .into_iter()
        .find(|revision| revision.revision_id == revision_id)
        .ok_or_else(|| {
            StorageError::Backpressure(
                "current belief view references a missing revision".to_string(),
            )
        })?;
    if revision.source_cursor_end == 0 {
        return Err(StorageError::InvalidPath(
            "belief revision source sequence must be positive for agent selection".to_string(),
        ));
    }
    Ok(Some(revision))
}

fn require_selected_revision(
    belief_query: &BeliefQuery<'_>,
    belief_key: &BeliefKey,
    revision_id: &str,
    revision_seq: u64,
) -> Result<(), StorageError> {
    let Some(revision) = current_revision(belief_query, belief_key)? else {
        return Err(stale("selected belief view is missing"));
    };
    if revision.revision_id != revision_id || revision.source_cursor_end != revision_seq {
        return Err(stale("selected belief revision changed"));
    }
    Ok(())
}

fn pending_revision(
    label: &str,
    cursor_revision_id: Option<&str>,
    cursor_seq: u64,
    revision: &BeliefRevision,
) -> Result<bool, StorageError> {
    if revision.source_cursor_end > cursor_seq {
        return Ok(true);
    }
    if revision.source_cursor_end == cursor_seq
        && cursor_revision_id == Some(revision.revision_id.as_str())
    {
        return Ok(false);
    }
    Err(StorageError::Backpressure(format!(
        "{label} cursor diverges from the current belief revision"
    )))
}

fn stale(message: &str) -> StorageError {
    StorageError::Backpressure(message.to_string())
}
