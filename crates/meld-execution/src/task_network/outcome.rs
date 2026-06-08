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
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Durable publication outbox entry for one task outcome.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Publication {
    /// Stable publication id.
    pub publication_id: String,
    /// Stable task network identifier.
    pub network_id: String,
    /// Task outcome represented by this publication.
    pub outcome: dispatch::Outcome,
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
            outcome: outcome.clone(),
            state: PublicationState::Pending,
        }
    }

    /// Returns the execution event type represented by this publication.
    pub fn event_type(&self) -> &'static str {
        event_type_for_status(&self.outcome.status)
    }

    /// Returns the execution event payload represented by this publication.
    pub fn event_payload(&self) -> Value {
        serde_json::to_value(&self.outcome).expect("task network outcome is serializable")
    }
}

impl<'de> Deserialize<'de> for Publication {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if value.get("outcome").is_some() {
            #[derive(Deserialize)]
            struct Wire {
                publication_id: String,
                network_id: String,
                outcome: dispatch::Outcome,
                state: PublicationState,
            }

            let wire: Wire = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            return Ok(Self {
                publication_id: wire.publication_id,
                network_id: wire.network_id,
                outcome: wire.outcome,
                state: wire.state,
            });
        }

        #[derive(Deserialize)]
        struct LegacyWire {
            publication_id: String,
            network_id: String,
            task_instance_id: String,
            outcome_id: String,
            event_type: String,
            event_payload: dispatch::Outcome,
            state: PublicationState,
        }

        let wire: LegacyWire = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        if wire.task_instance_id != wire.event_payload.task_instance_id {
            return Err(serde::de::Error::custom(
                "legacy publication task identity mismatch",
            ));
        }
        if wire.outcome_id != wire.event_payload.outcome_id {
            return Err(serde::de::Error::custom(
                "legacy publication outcome identity mismatch",
            ));
        }
        if wire.event_type != event_type_for_status(&wire.event_payload.status) {
            return Err(serde::de::Error::custom(
                "legacy publication event type mismatch",
            ));
        }

        Ok(Self {
            publication_id: wire.publication_id,
            network_id: wire.network_id,
            outcome: wire.event_payload,
            state: wire.state,
        })
    }
}

fn event_type_for_status(status: &dispatch::OutcomeStatus) -> &'static str {
    match status {
        dispatch::OutcomeStatus::Succeeded => "execution.task.succeeded",
        dispatch::OutcomeStatus::Failed => "execution.task.failed",
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
