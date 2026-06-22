//! Runtime actor facades for task network bounded work.
//!
//! This module exposes execution actor surfaces over task network operations
//! without moving command reduction, publication selection, or event append
//! semantics out of their owning modules.

use crate::task_network::{
    publication::{
        publish_pending_publications, EventAppendSink, PublicationBridgeError,
        PublicationBridgeIssue, PublicationBridgeReport, PublicationBridgeScope,
        PublicationPublishResult, PublishPendingPublicationsRequest,
    },
    store::SledTaskNetworkStore,
};

const PUBLICATION_RUNTIME_ACTOR_ID: &str = "execution.task_network.publication.runtime";

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
        store: &mut SledTaskNetworkStore,
        events: &E,
        request: PublishPendingPublicationsRequest,
    ) -> Result<PublicationRuntimeReport, PublicationBridgeError>
    where
        E: EventAppendSink,
    {
        let bridge_report = publish_pending_publications(store, events, request)?;
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
