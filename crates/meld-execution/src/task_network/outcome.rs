//! Task outcome publication outbox contracts.
//!
//! Owner: task network.
//! Inputs: completed task outcomes and publication mark commands.
//! Outputs: durable publication records and publication state transitions.
//! Does not own: this module does not append to external event stores.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::outcome::PublicationState;
//!
//! let state = PublicationState::Pending;
//! assert!(matches!(state, PublicationState::Pending));
//! ```

use crate::task_network::{contracts::stable_id, dispatch};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Durable publication outbox entry for one task outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Publication {
    /// Stable publication id.
    pub publication_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Task instance that produced the outcome.
    pub task_instance_id: String,
    /// Outcome id represented by this publication.
    pub outcome_id: String,
    /// Event type to publish through the execution event surface.
    pub event_type: String,
    /// Serialized event payload.
    pub event_payload: Value,
    /// Current publication state.
    pub state: PublicationState,
}

impl Publication {
    /// Creates a pending publication record for a task outcome.
    pub fn pending_for_outcome(network_id: &str, outcome: &dispatch::Outcome) -> Self {
        #[derive(Serialize)]
        struct Identity<'a> {
            network_id: &'a str,
            outcome_id: &'a str,
            task_instance_id: &'a str,
        }

        let event_type = match outcome.status {
            dispatch::OutcomeStatus::Succeeded => "execution.task.succeeded",
            dispatch::OutcomeStatus::Failed => "execution.task.failed",
        }
        .to_string();

        Self {
            publication_id: stable_id(
                "task-network-publication",
                &Identity {
                    network_id,
                    outcome_id: &outcome.outcome_id,
                    task_instance_id: &outcome.task_instance_id,
                },
            ),
            network_id: network_id.to_string(),
            task_instance_id: outcome.task_instance_id.clone(),
            outcome_id: outcome.outcome_id.clone(),
            event_type,
            event_payload: serde_json::to_value(outcome)
                .expect("task network outcome is serializable"),
            state: PublicationState::Pending,
        }
    }
}

/// Durable publication lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicationState {
    /// Publication is waiting for an external append attempt.
    Pending,
    /// Publication was appended successfully at a task network revision.
    Published {
        /// Revision that marked the publication as complete.
        marked_revision: u64,
    },
    /// Last publication attempt failed and can be retried.
    Failed {
        /// Failure summary from the publication worker.
        error: String,
    },
}
