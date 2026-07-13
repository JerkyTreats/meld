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
use meld_events::AppendReceipt;
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
    /// Semantic lineage for canonical attributed outcomes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_lineage: Option<dispatch::OutcomeSemanticLineage>,
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
            semantic_lineage: None,
            state: PublicationState::Pending,
        }
    }

    /// Creates a pending publication with complete semantic attribution.
    pub fn pending_for_attributed_outcome(
        network_id: &str,
        attributed: &dispatch::AttributedOutcome,
    ) -> Self {
        let mut publication = Self::pending_for_outcome(network_id, &attributed.outcome);
        publication.semantic_lineage = Some(attributed.semantic_lineage.clone());
        publication
    }

    /// Returns the execution event type represented by this publication.
    pub fn event_type(&self) -> &'static str {
        event_type_for_status(&self.outcome.status)
    }

    /// Returns the execution event payload represented by this publication.
    pub fn event_payload(&self) -> Value {
        let mut payload = serde_json::to_value(&self.outcome)
            .expect("task network outcome is serializable for publication");
        if let Some(lineage) = &self.semantic_lineage {
            payload
                .as_object_mut()
                .expect("task network outcome serializes as an object")
                .insert(
                    "semantic_lineage".to_string(),
                    serde_json::to_value(lineage)
                        .expect("task outcome lineage is serializable for publication"),
                );
        }
        payload
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
                #[serde(default)]
                semantic_lineage: Option<dispatch::OutcomeSemanticLineage>,
                state: PublicationState,
            }

            let wire: Wire = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            return Ok(Self {
                publication_id: wire.publication_id,
                network_id: wire.network_id,
                outcome: wire.outcome,
                semantic_lineage: wire.semantic_lineage,
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
            semantic_lineage: None,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PublicationState {
    /// Publication is waiting for an external append attempt.
    Pending,
    /// Publication was appended successfully at a task network revision.
    Published {
        /// Revision that marked the publication as complete.
        marked_revision: u64,
        /// Complete identity-bearing authority acknowledgement.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        receipt: Option<AppendReceipt>,
        /// Pre-authority sequence retained only to replay historical state
        /// hashes exactly until the receipt is upgraded.
        // TODO compat-shim: remove after the minimum supported persisted task-network
        // schema guarantees identity-bearing receipts. Until then,
        // task_network_publication_bridge::persisted_legacy_receipt_reopens_and_upgrades
        // proves event_seq-only state is decoded, retried, and durably upgraded.
        #[serde(rename = "event_seq")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        legacy_event_seq: Option<u64>,
    },
    /// Last publication attempt failed and can be retried.
    Failed {
        /// Failure summary from the last publication attempt.
        error: String,
    },
}

impl<'de> Deserialize<'de> for PublicationState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Wire {
            Pending,
            Published {
                marked_revision: u64,
                #[serde(default)]
                receipt: Option<AppendReceipt>,
                #[serde(default)]
                event_seq: Option<u64>,
            },
            Failed {
                error: String,
            },
        }

        let value = Value::deserialize(deserializer)?;
        let published_fields = value.get("Published").and_then(Value::as_object);
        let receipt_field_present =
            published_fields.is_some_and(|published| published.contains_key("receipt"));
        let event_seq_field_present =
            published_fields.is_some_and(|published| published.contains_key("event_seq"));
        let wire: Wire = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        match wire {
            Wire::Pending => Ok(Self::Pending),
            Wire::Failed { error } => Ok(Self::Failed { error }),
            Wire::Published {
                marked_revision,
                receipt,
                event_seq,
            } => {
                if receipt_field_present && receipt.is_none() {
                    return Err(serde::de::Error::custom(
                        "canonical publication receipt must be complete",
                    ));
                }
                if receipt.is_some() && event_seq_field_present {
                    return Err(serde::de::Error::custom(
                        "publication state cannot contain canonical receipt and legacy event_seq",
                    ));
                }
                Ok(Self::Published {
                    marked_revision,
                    receipt,
                    legacy_event_seq: event_seq,
                })
            }
        }
    }
}
