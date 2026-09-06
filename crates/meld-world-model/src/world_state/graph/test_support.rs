//! Explicit test fixtures for graph projection characterization.
//!
//! Production composition must use [`super::runtime::GraphRuntime::from_ports`]
//! with application-owned ports. These helpers keep authority setup and frozen
//! raw-ledger replay out of production constructors while allowing recovery and
//! compatibility tests to exercise the same projection code.

use std::sync::Arc;

use meld_events::error::EventAuthorityError;

use crate::error::StorageError;
use crate::events::{
    AppendMode, AppendReceipt, EventAppendCapability, EventAuthority, EventAuthorityOpenOptions,
    EventConsumerRegistryCapability, EventEnvelope, EventPage, EventReplayCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};

use super::ports::{GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource};
use super::runtime::GraphRuntime;
use super::store::TraversalStore;

const GRAPH_ACTOR_ID: &str = "world_state.graph.reducer";

#[derive(Clone)]
struct AuthorityGraphTestPorts {
    replay: EventReplayCapability,
    append: EventAppendCapability,
    registry: EventConsumerRegistryCapability,
}

impl AuthorityGraphTestPorts {
    fn new(authority: &EventAuthority) -> Self {
        Self {
            replay: authority.replay_capability(),
            append: authority.append_capability(),
            registry: authority.consumer_registry_capability(),
        }
    }
}

impl GraphEventReplaySource for AuthorityGraphTestPorts {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl GraphDerivedEventSink for AuthorityGraphTestPorts {
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

impl GraphConsumerCursorReporter for AuthorityGraphTestPorts {
    fn report_owner_source_cursor(
        &self,
        source: &crate::world_state::graph::admission::OwnerEventSourceRef,
        cursor: LedgerCursor,
    ) -> Result<(), EventAuthorityError> {
        self.registry
            .report(&source.consumer_id(), cursor)
            .map(|_| ())
    }

    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry.report(GRAPH_ACTOR_ID, cursor)?;
        Ok(())
    }
}

/// Authority-backed graph fixture for crate and downstream integration tests.
pub struct GraphRuntimeTestFixture {
    authority: EventAuthority,
    runtime: Arc<GraphRuntime>,
}

impl GraphRuntimeTestFixture {
    /// Open one event authority and graph projection over a shared test database.
    pub fn open(db: sled::Db) -> Result<Self, StorageError> {
        let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default())
            .map_err(authority_error_to_storage)?;
        let ports = Arc::new(AuthorityGraphTestPorts::new(&authority));
        let traversal = TraversalStore::shared(db)?;
        let runtime = Arc::new(GraphRuntime::from_ports(
            ports.clone(),
            ports.clone(),
            ports,
            traversal,
        )?);
        Ok(Self { authority, runtime })
    }

    /// Shared graph runtime under test.
    pub fn runtime(&self) -> Arc<GraphRuntime> {
        Arc::clone(&self.runtime)
    }

    /// Ledger identity shared by all fixture capabilities.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.authority.ledger_identity()
    }

    /// Append one canonical source event through the authority capability.
    pub fn append(&self, envelope: EventEnvelope) -> Result<AppendReceipt, EventAuthorityError> {
        self.authority
            .append_capability()
            .append_durable(envelope, AppendMode::Plain)
    }

    /// Replay all currently retained canonical records through the authority.
    pub fn records(&self) -> Result<Vec<crate::events::EventRecord>, EventAuthorityError> {
        let replay = self.authority.replay_capability();
        let mut cursor = LedgerCursor {
            ledger_id: replay.ledger_identity(),
            after_seq: 0,
        };
        let mut records = Vec::new();
        loop {
            let page = replay.replay(ReplayRequest {
                cursor,
                limit: crate::events::MAX_REPLAY_LIMIT,
            })?;
            let done = page.records.is_empty()
                || !matches!(
                    page.coverage.truncation,
                    crate::events::CoverageTruncation::After
                        | crate::events::CoverageTruncation::Both
                );
            cursor = page.next_cursor;
            records.extend(page.records);
            if done {
                return Ok(records);
            }
        }
    }
}

fn authority_error_to_storage(error: EventAuthorityError) -> StorageError {
    match error {
        EventAuthorityError::InvalidRequest { message }
        | EventAuthorityError::Internal { message }
        | EventAuthorityError::CorruptPersistedIdentity { message }
        | EventAuthorityError::MigrationConflict { message } => StorageError::InvalidPath(message),
        EventAuthorityError::IdentityMismatch { expected, actual } => {
            StorageError::IdentityMismatch { expected, actual }
        }
        EventAuthorityError::RetentionGap {
            after_seq,
            retained_from,
            ..
        } => StorageError::RetentionGap {
            after_seq,
            retained_from,
        },
        EventAuthorityError::Backpressure { message } => StorageError::Backpressure(message),
        EventAuthorityError::DurabilityIndeterminate { message } => {
            StorageError::DurabilityIndeterminate(message)
        }
        EventAuthorityError::Unavailable { message } => StorageError::Unavailable(message),
        EventAuthorityError::Persistence { message } => {
            StorageError::IoError(std::io::Error::other(message))
        }
        EventAuthorityError::DuplicateAuthorityBinding { ledger_id } => StorageError::IoError(
            std::io::Error::other(format!("duplicate authority binding for {ledger_id}")),
        ),
    }
}
