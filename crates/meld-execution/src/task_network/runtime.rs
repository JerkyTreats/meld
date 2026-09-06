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
use crate::waiting::{conditions, StructuralWakeAddress, WaitingOnDeclaration};

const PUBLICATION_RUNTIME_ACTOR_ID: &str = "execution.task_network.publication.runtime";

/// Bounded actor facade for retryable task network publication outbox work.
#[derive(Debug, Clone)]
pub struct PublicationRuntime {
    actor_id: String,
    lifecycle: crate::lifecycle::NativeLifecycle,
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
            lifecycle: crate::lifecycle::NativeLifecycle::new(PUBLICATION_RUNTIME_ACTOR_ID),
        }
    }

    /// Resolve only the publication outbox of the bound Task Network.
    pub fn resolves_wake(
        &self,
        network: &SledTaskNetworkStore,
        wake: &StructuralWakeAddress,
    ) -> bool {
        matches!(wake, StructuralWakeAddress::DurableOperation(value)
            if crate::waiting::after_position(value, &format!("publication-outbox::{}", network.state().network_id)))
    }

    /// Account for the native durable work in the borrowed, exclusively bound network.
    pub fn lifecycle_evidence<E: EventAppendSink>(
        &self,
        network: &crate::task_network::store::SledTaskNetworkStore,
        events: &E,
        binding: &PublishPendingPublicationsRequest,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        if binding.worker_id.trim().is_empty() || binding.session_id.trim().is_empty() {
            return Err("Publication requires an exact worker and Event session".into());
        }
        network.flush().map_err(|error| error.to_string())?;
        let state = network.state();
        let pending: Vec<_> = state
            .publications
            .values()
            .filter(|publication| {
                matches!(
                    publication.state,
                    crate::task_network::outcome::PublicationState::Pending
                )
            })
            .collect();
        let checkpoint_ref = format!(
            "publication-outbox::{}::{}",
            state.network_id, state.revision
        );
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: vec![format!("task-network-state::{}", state.state_hash)],
            binding_refs: vec![
                format!("publication-owner::{}", self.actor_id),
                format!(
                    "publication-worker::{}::{}",
                    binding.worker_id, binding.session_id
                ),
                format!("task-network::{}", state.network_id),
                format!("event-ledger::{}", events.ledger_identity()),
            ],
            subscription_refs: vec![format!("publication-outbox::{}", state.network_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "publication-durable-pending",
                &pending,
            )?,
        })
    }

    /// Author native start evidence while the network is borrowed.
    pub fn lifecycle_start<E: EventAppendSink>(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
        events: &E,
        binding: &PublishPendingPublicationsRequest,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network, events, binding)?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence while the network is borrowed.
    pub fn lifecycle_safe_point<E: EventAppendSink>(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
        events: &E,
        binding: &PublishPendingPublicationsRequest,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network, events, binding)?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence while the network is borrowed.
    pub fn lifecycle_stop<E: EventAppendSink>(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
        events: &E,
        binding: &PublishPendingPublicationsRequest,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network, events, binding)?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence while the network is borrowed.
    pub fn lifecycle_release<E: EventAppendSink>(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
        events: &E,
        binding: &PublishPendingPublicationsRequest,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network, events, binding)?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
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
    /// Owner-authored conditions that can make publication eligible again.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

impl PublicationRuntimeReport {
    fn from_bridge(actor_id: String, report: PublicationBridgeReport) -> Self {
        let waiting_on = if report.items_attempted == 0
            && report.retryable_errors.is_empty()
            && report.fatal_errors.is_empty()
        {
            vec![WaitingOnDeclaration::broad(
                conditions::NO_PENDING_PUBLICATIONS,
                format!(
                    "no pending Task Network publications at revision {}",
                    report.output_revision
                ),
                vec![StructuralWakeAddress::DurableOperation(format!(
                    "publication-outbox::{}::after::{}",
                    report.scope.network_id, report.output_revision
                ))],
            )]
        } else {
            Vec::new()
        };
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
            waiting_on,
        }
    }
}
