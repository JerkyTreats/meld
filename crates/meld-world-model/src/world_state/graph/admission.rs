//! Native admission of owner-defined Event kinds using the shared publication grammar.

use serde::{Deserialize, Serialize};

use crate::belief::TheoryRevisionRef;
use crate::error::StorageError;

pub const OWNER_EVENT_ROUTE_REGISTRY: &str = "graph_owner_event_route";

/// Exact owner route. Graph interprets the structural publication schema only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphOwnerEventRoute {
    /// The owner declares this route exhaustive for its publication scopes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub complete_event_source: bool,
    pub route_id: String,
    pub owner_id: String,
    pub event_type: String,
    pub enumeration_rule_revision: String,
}

impl GraphOwnerEventRoute {
    pub fn validate(&self) -> Result<(), StorageError> {
        if [
            &self.route_id,
            &self.owner_id,
            &self.event_type,
            &self.enumeration_rule_revision,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(StorageError::InvalidPath(
                "Graph owner Event route is incomplete".into(),
            ));
        }
        if self.event_type == super::contracts::OWNER_PUBLICATION_EVENT_TYPE {
            return Err(StorageError::InvalidPath(
                "the standard owner publication route cannot be replaced".into(),
            ));
        }
        Ok(())
    }

    pub fn revision_ref(&self) -> Result<TheoryRevisionRef, StorageError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        Ok(TheoryRevisionRef {
            registry: OWNER_EVENT_ROUTE_REGISTRY.into(),
            id: self.route_id.clone(),
            content_hash: blake3::hash(&bytes).to_hex().to_string(),
        })
    }
}

/// Exact immutable owner route selected as an exhaustive Event source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerEventSourceRef {
    pub route_id: String,
    pub content_hash: String,
}

impl GraphOwnerEventRoute {
    pub fn source_ref(&self) -> Result<OwnerEventSourceRef, StorageError> {
        let reference = self.revision_ref()?;
        Ok(OwnerEventSourceRef {
            route_id: reference.id,
            content_hash: reference.content_hash,
        })
    }
}

impl OwnerEventSourceRef {
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.route_id.trim().is_empty() || self.content_hash.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "Event source requires an exact owner route revision".into(),
            ));
        }
        Ok(())
    }
}

/// Durable inspection position for an owner route's historical Event replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerEventReplayState {
    pub source: OwnerEventSourceRef,
    pub position: meld_events::LedgerCursor,
    pub covered: bool,
}

impl OwnerEventSourceRef {
    pub fn consumer_id(&self) -> String {
        format!(
            "world_state.graph.owner-source::{}::{}",
            self.route_id, self.content_hash
        )
    }
}
