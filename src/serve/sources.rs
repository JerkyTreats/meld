//! Owned read surfaces behind the served substrate.
//!
//! The sources own their handles (Arc'd stores, cloned ports, sibling
//! sled trees) so the listener thread shares nothing borrowed from the
//! foreground tick loop; sled and the event authority are safe for
//! concurrent same-process readers. Live and playback differ only in how
//! the sources are built — the handlers cannot tell them apart, which is
//! what makes the two surfaces byte-consistent.

use std::sync::Arc;

use meld_events::remote::LocalEventAuthorityClient;
use meld_events::LedgerIdentity;
use meld_execution::goals::PersistentGoalSetStore;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_world_model::agent::AgentStore;
use meld_world_model::belief::BeliefStore;
use meld_world_model::world_state::graph::store::TraversalStore;

use crate::harness::boot::HarnessError;
use crate::harness::eligibility::EligibilityWalker;
use crate::harness::walk::ThreadWalker;
use crate::runtime::assembly::ProductRuntimeAssembly;
use crate::runtime::ports::ProductEventReplayPort;
use crate::runtime::supervisor::SupervisorReportStore;

/// Owned read surfaces for one served root.
pub struct ServeSources {
    pub(crate) events: LocalEventAuthorityClient,
    pub(crate) ledger_id: LedgerIdentity,
    pub(crate) reports: SupervisorReportStore,
    pub(crate) replay_port: ProductEventReplayPort,
    pub(crate) belief: Option<Arc<BeliefStore>>,
    pub(crate) agent: Option<Arc<AgentStore>>,
    pub(crate) traversal: Option<Arc<TraversalStore>>,
    pub(crate) goals: Option<Arc<PersistentGoalSetStore>>,
    pub(crate) task_networks: Option<TaskNetworkStoreFactory>,
}

impl ServeSources {
    /// Build sources over a live composition.
    ///
    /// Store handles are cloned Arcs and sibling sled trees over the same
    /// database the running process owns, so the served surface answers
    /// while the run is live without a second file lock.
    pub fn from_assembly(assembly: &ProductRuntimeAssembly) -> Result<Self, HarnessError> {
        let authority = assembly.event_authority();
        let stores = assembly.stores();
        Ok(Self {
            events: LocalEventAuthorityClient::new(authority.as_ref()),
            ledger_id: authority.ledger_identity(),
            reports: SupervisorReportStore::open(assembly.supervisor_store())
                .map_err(|error| HarnessError::Storage(error.to_string()))?,
            replay_port: assembly.ports().event_replay().clone(),
            belief: stores.belief_store.opened().map(Arc::clone),
            agent: stores.agent_store.opened().map(Arc::clone),
            traversal: stores.traversal_store.opened().map(Arc::clone),
            goals: stores.goal_store.opened().map(Arc::clone),
            task_networks: stores.task_networks.opened().cloned(),
        })
    }

    /// The ledger identity every request must address.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    pub(crate) fn thread_walker(&self) -> ThreadWalker<'_> {
        ThreadWalker::from_parts(
            Some(&self.replay_port),
            self.belief.as_deref(),
            self.agent.as_deref(),
            self.traversal.as_deref(),
            self.goals.as_deref(),
            self.task_networks.as_ref(),
        )
    }

    pub(crate) fn projection_sources(&self) -> crate::harness::projections::ProjectionSources<'_> {
        crate::harness::projections::ProjectionSources {
            reports: &self.reports,
            replay: Some(&self.replay_port),
            belief: self.belief.as_deref(),
        }
    }

    pub(crate) fn eligibility_walker(&self) -> EligibilityWalker<'_> {
        let walker = EligibilityWalker::new(&self.reports);
        match self.traversal.as_deref() {
            Some(traversal) => walker.with_traversal(traversal),
            None => walker,
        }
    }
}
