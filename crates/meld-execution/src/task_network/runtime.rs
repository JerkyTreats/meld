//! Runtime actor facades for task network bounded work.
//!
//! This module exposes execution actor surfaces over task network operations
//! without moving command reduction, publication selection, or event append
//! semantics out of their owning modules.

use crate::activation::ExecutionActivationSelection;
use crate::task_network::{
    authority::{TaskNetworkCommandPort, TaskNetworkQueryPort},
    publication::{
        publish_pending_publications, EventAppendSink, PublicationBridgeError,
        PublicationBridgeIssue, PublicationBridgeReport, PublicationBridgeScope,
        PublicationPublishResult, PublishPendingPublicationsRequest,
    },
};

const PUBLICATION_RUNTIME_ACTOR_ID: &str = "execution.publication";
const PUBLICATION_SESSION_DOMAIN: &[u8] = b"meld.execution.publication-session.v1";

/// Stable publication scope derived by the execution domain from activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationRuntimeScope {
    /// Task network whose durable publication outbox is consumed.
    pub network_id: String,
    /// Stable event-ledger session reused across actor replacement.
    pub session_id: String,
}

impl PublicationRuntimeScope {
    /// Derive a replacement-stable scope from accepted execution activation.
    pub fn from_activation(
        selection: &ExecutionActivationSelection,
    ) -> Result<Self, PublicationBridgeError> {
        if selection.activation_id.trim().is_empty()
            || selection.activation_hash.trim().is_empty()
            || selection.task_network_id.trim().is_empty()
            || selection.publication.mapping_id.trim().is_empty()
        {
            return Err(PublicationBridgeError::InvalidRequest(
                "publication activation identity must be complete".to_string(),
            ));
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(PUBLICATION_SESSION_DOMAIN);
        for field in [
            selection.activation_id.as_bytes(),
            selection.activation_hash.as_bytes(),
            selection.task_network_id.as_bytes(),
            selection.publication.mapping_id.as_bytes(),
        ] {
            hasher.update(&(field.len() as u64).to_be_bytes());
            hasher.update(field);
        }
        Ok(Self {
            network_id: selection.task_network_id.clone(),
            session_id: format!("publication-{}", hasher.finalize().to_hex()),
        })
    }
}

/// Bounded actor facade for retryable task network publication outbox work.
#[derive(Debug, Clone)]
pub struct PublicationRuntime {
    actor_id: String,
}

impl Default for PublicationRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl PublicationRuntime {
    /// Create a publication runtime using the stable actor id.
    pub fn new() -> Self {
        Self {
            actor_id: PUBLICATION_RUNTIME_ACTOR_ID.to_string(),
        }
    }

    /// Return the stable actor id used in reports.
    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Publish a bounded set of retryable outbox records through an append sink.
    pub fn publish_pending<E>(
        &self,
        query: &TaskNetworkQueryPort,
        commands: &TaskNetworkCommandPort,
        events: &E,
        request: PublishPendingPublicationsRequest,
    ) -> Result<PublicationRuntimeReport, PublicationBridgeError>
    where
        E: EventAppendSink,
    {
        let bridge_report = publish_pending_publications(query, commands, events, request)?;
        Ok(PublicationRuntimeReport::from_bridge(
            self.actor_id.clone(),
            bridge_report,
        ))
    }
}

/// Report returned by one bounded publication actor pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationRuntimeReport {
    /// Stable runtime actor identifier.
    pub actor_id: String,
    /// Domain-specific report scope.
    pub scope: PublicationBridgeScope,
    /// Task network revision before selecting publications.
    pub input_revision: u64,
    /// Task network revision after processing selected publications.
    pub output_revision: u64,
    /// Publication records attempted by this pass.
    pub attempted: usize,
    /// Publication records successfully appended and marked published.
    pub committed: usize,
    /// Retryable diagnostics observed during the pass.
    pub retryable_errors: Vec<PublicationBridgeIssue>,
    /// Fatal diagnostics observed during the pass.
    pub fatal_errors: Vec<PublicationBridgeIssue>,
    /// True when more retryable publication records remain after the limit.
    pub budget_exhausted: bool,
    /// Per publication results in deterministic publication id order.
    pub results: Vec<PublicationPublishResult>,
}

impl PublicationRuntimeReport {
    fn from_bridge(actor_id: String, report: PublicationBridgeReport) -> Self {
        Self {
            actor_id,
            scope: report.scope,
            input_revision: report.input_revision,
            output_revision: report.output_revision,
            attempted: report.items_attempted,
            committed: report.items_committed,
            retryable_errors: report.retryable_errors,
            fatal_errors: report.fatal_errors,
            budget_exhausted: report.budget_exhausted,
            results: report.results,
        }
    }
}
