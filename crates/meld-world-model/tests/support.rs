use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, AppendReceipt, EventAppendCapability, EventAuthority, EventAuthorityOpenOptions,
    EventConsumerRegistryCapability, EventEnvelope, EventPage, EventReplayCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_world_model::error::StorageError;
use meld_world_model::world_state::graph::runtime::GraphRuntime;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource,
};

const GRAPH_ACTOR_ID: &str = "world_state.graph.reducer";

#[derive(Clone)]
struct TestGraphPorts {
    replay: EventReplayCapability,
    append: EventAppendCapability,
    registry: EventConsumerRegistryCapability,
}

impl GraphEventReplaySource for TestGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl GraphDerivedEventSink for TestGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_derived(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.append.append_durable(envelope, AppendMode::Idempotent)
    }
}

impl GraphConsumerCursorReporter for TestGraphPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry.report(GRAPH_ACTOR_ID, cursor)?;
        Ok(())
    }
}

pub struct GraphRuntimeTestFixture {
    authority: EventAuthority,
    runtime: Arc<GraphRuntime>,
}

impl GraphRuntimeTestFixture {
    pub fn open(db: sled::Db) -> Result<Self, StorageError> {
        let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default())
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let ports = Arc::new(TestGraphPorts {
            replay: authority.replay_capability(),
            append: authority.append_capability(),
            registry: authority.consumer_registry_capability(),
        });
        let traversal = TraversalStore::shared(db)?;
        let runtime = Arc::new(GraphRuntime::from_ports(
            ports.clone(),
            ports.clone(),
            ports,
            traversal,
        )?);
        Ok(Self { authority, runtime })
    }

    pub fn runtime(&self) -> Arc<GraphRuntime> {
        Arc::clone(&self.runtime)
    }

    pub fn append(&self, envelope: EventEnvelope) -> Result<AppendReceipt, EventAuthorityError> {
        self.authority
            .append_capability()
            .append_durable(envelope, AppendMode::Plain)
    }

    pub fn records(&self) -> Result<Vec<meld_events::EventRecord>, EventAuthorityError> {
        let replay = self.authority.replay_capability();
        let page = replay.replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: replay.ledger_identity(),
                after_seq: 0,
            },
            limit: meld_events::MAX_REPLAY_LIMIT,
        })?;
        Ok(page.records)
    }
}
