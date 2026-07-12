//! Explicit low-level event fixtures for tests, benchmarks, and fuzz targets.
//!
//! Production code opens an [`EventAuthority`](crate::events::EventAuthority)
//! and consumes its derived capabilities. This feature-gated module is the
//! only external path to raw stores, writers, cursors, and observability
//! backings needed to characterize the frozen persistence formats.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::identity::LedgerIdentity;

pub use super::observability::LedgerObservability;
pub use super::registry::EventCursorRegistry;
pub use super::runtime::EventRuntime;
pub use super::store::EventStore;
pub use super::subscription::{EventCursor, EventSubscription};
pub use super::writer::{CommitWatermark, EventWriter};

/// Restores the frozen raw-store syntax only in explicit test-support builds.
pub trait EventStoreTestSupport: Sized {
    /// Opens a raw store.
    fn new(db: Db) -> Result<Self, StorageError>;
    /// Opens a shared raw store.
    fn shared(db: Db) -> Result<Arc<Self>, StorageError>;
    /// Returns the raw database handle.
    fn db(&self) -> &Db;
    /// Establishes or reads the raw fixture's persisted ledger identity.
    fn compatibility_ledger_identity(&self) -> Result<LedgerIdentity, StorageError>;
}

impl EventStoreTestSupport for EventStore {
    fn new(db: Db) -> Result<Self, StorageError> {
        EventStore::new(db)
    }

    fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        EventStore::shared(db)
    }

    fn db(&self) -> &Db {
        self.db()
    }

    fn compatibility_ledger_identity(&self) -> Result<LedgerIdentity, StorageError> {
        self.compatibility_ledger_identity()
    }
}

/// Restores the legacy runtime syntax only in explicit test-support builds.
pub trait EventRuntimeTestSupport: Sized {
    /// Opens a compatibility runtime.
    fn new(db: Db) -> Result<Self, StorageError>;
    /// Opens a compatibility runtime over a raw store.
    fn from_store(store: Arc<EventStore>) -> Self;
    /// Returns its raw store.
    fn store(&self) -> &EventStore;
    /// Returns its raw commit watermark.
    fn watermark(&self) -> Arc<CommitWatermark>;
    /// Returns its raw drop counter.
    fn dropped_handle(&self) -> Arc<AtomicU64>;
}

impl EventRuntimeTestSupport for EventRuntime {
    fn new(db: Db) -> Result<Self, StorageError> {
        EventRuntime::new(db)
    }

    fn from_store(store: Arc<EventStore>) -> Self {
        EventRuntime::from_store(store)
    }

    fn store(&self) -> &EventStore {
        self.store()
    }

    fn watermark(&self) -> Arc<CommitWatermark> {
        self.watermark()
    }

    fn dropped_handle(&self) -> Arc<AtomicU64> {
        self.dropped_handle()
    }
}

/// Restores raw writer construction and handles in test-support builds.
pub trait EventWriterTestSupport: Sized {
    /// Spawns a raw writer.
    fn spawn(store: Arc<EventStore>) -> Self;
    /// Returns its commit watermark.
    fn watermark(&self) -> Arc<CommitWatermark>;
    /// Returns its drop counter.
    fn dropped_handle(&self) -> Arc<AtomicU64>;
}

impl EventWriterTestSupport for EventWriter {
    fn spawn(store: Arc<EventStore>) -> Self {
        EventWriter::spawn(store)
    }

    fn watermark(&self) -> Arc<CommitWatermark> {
        self.watermark()
    }

    fn dropped_handle(&self) -> Arc<AtomicU64> {
        self.dropped_handle()
    }
}

/// Restores raw subscription construction and handles in test builds.
pub trait EventSubscriptionTestSupport: Sized {
    /// Binds a raw subscription.
    fn new(store: Arc<EventStore>, watermark: Arc<CommitWatermark>) -> Self;
    /// Returns the matched raw watermark.
    fn watermark(&self) -> Arc<CommitWatermark>;
}

impl EventSubscriptionTestSupport for EventSubscription {
    fn new(store: Arc<EventStore>, watermark: Arc<CommitWatermark>) -> Self {
        EventSubscription::new(store, watermark)
    }

    fn watermark(&self) -> Arc<CommitWatermark> {
        self.watermark()
    }
}

/// Restores raw cursor binding only in test-support builds.
pub trait EventCursorTestSupport: Sized {
    /// Binds an unlabelled legacy cursor.
    fn new(tree: Tree, name: impl AsRef<str>) -> Self;
    /// Binds an identity-bearing compatibility cursor.
    fn bind_compatibility(tree: Tree, name: impl AsRef<str>, ledger_id: LedgerIdentity) -> Self;
    /// Migrates a proven legacy cursor.
    fn migrate_legacy(
        tree: Tree,
        name: impl AsRef<str>,
        ledger_id: LedgerIdentity,
    ) -> Result<Self, StorageError>;
}

impl EventCursorTestSupport for EventCursor {
    fn new(tree: Tree, name: impl AsRef<str>) -> Self {
        EventCursor::new(tree, name)
    }

    fn bind_compatibility(tree: Tree, name: impl AsRef<str>, ledger_id: LedgerIdentity) -> Self {
        EventCursor::bind_compatibility(tree, name, ledger_id)
    }

    fn migrate_legacy(
        tree: Tree,
        name: impl AsRef<str>,
        ledger_id: LedgerIdentity,
    ) -> Result<Self, StorageError> {
        EventCursor::migrate_legacy(tree, name, ledger_id)
    }
}

/// Restores raw registry construction only in test-support builds.
pub trait EventCursorRegistryTestSupport: Sized {
    /// Opens an unbound legacy registry.
    fn open(db: &Db) -> Result<Self, StorageError>;
    /// Migrates proven legacy payloads into an identity-bound registry.
    fn migrate_legacy_payloads(db: &Db, ledger_id: LedgerIdentity) -> Result<Self, StorageError>;
}

impl EventCursorRegistryTestSupport for EventCursorRegistry {
    fn open(db: &Db) -> Result<Self, StorageError> {
        EventCursorRegistry::open(db)
    }

    fn migrate_legacy_payloads(db: &Db, ledger_id: LedgerIdentity) -> Result<Self, StorageError> {
        EventCursorRegistry::migrate_legacy_payloads(db, ledger_id)
    }
}

/// Restores raw observability composition only in test-support builds.
pub trait LedgerObservabilityTestSupport: Sized {
    /// Binds a valid raw observability fixture.
    fn new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Self;
    /// Fallibly binds a raw observability fixture.
    fn try_new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Result<Self, StorageError>;
}

impl LedgerObservabilityTestSupport for LedgerObservability {
    fn new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Self {
        LedgerObservability::new(store, watermark, registry, dropped)
    }

    fn try_new(
        store: Arc<EventStore>,
        watermark: Arc<CommitWatermark>,
        registry: EventCursorRegistry,
        dropped: Arc<AtomicU64>,
    ) -> Result<Self, StorageError> {
        LedgerObservability::try_new(store, watermark, registry, dropped)
    }
}

/// Opens a raw ledger for frozen-format characterization.
pub fn open_store(db: Db) -> Result<EventStore, StorageError> {
    EventStore::new(db)
}

/// Opens a shared raw ledger for concurrency characterization.
pub fn shared_store(db: Db) -> Result<Arc<EventStore>, StorageError> {
    EventStore::shared(db)
}

/// Clones the raw database handle owned by a test store.
pub fn store_db(store: &EventStore) -> Db {
    store.db().clone()
}

/// Opens the legacy runtime facade for compatibility characterization.
pub fn open_runtime(db: Db) -> Result<EventRuntime, StorageError> {
    EventRuntime::new(db)
}

/// Returns the raw store behind a compatibility runtime.
pub fn runtime_store(runtime: &EventRuntime) -> &EventStore {
    runtime.store()
}

/// Returns the raw watermark behind a compatibility runtime.
pub fn runtime_watermark(runtime: &EventRuntime) -> Arc<CommitWatermark> {
    runtime.watermark()
}

/// Spawns a raw writer for group-commit characterization.
pub fn spawn_writer(store: Arc<EventStore>) -> EventWriter {
    EventWriter::spawn(store)
}

/// Returns a raw writer watermark for low-level wait tests.
pub fn writer_watermark(writer: &EventWriter) -> Arc<CommitWatermark> {
    writer.watermark()
}

/// Returns the raw writer drop counter for observability characterization.
pub fn writer_dropped_handle(writer: &EventWriter) -> Arc<AtomicU64> {
    writer.dropped_handle()
}

/// Binds a raw subscription to a matched test store and writer watermark.
pub fn subscription(store: Arc<EventStore>, watermark: Arc<CommitWatermark>) -> EventSubscription {
    EventSubscription::new(store, watermark)
}

/// Opens an unbound legacy consumer registry for migration characterization.
pub fn open_registry(db: &Db) -> Result<EventCursorRegistry, StorageError> {
    EventCursorRegistry::open(db)
}

/// Explicitly migrates legacy registry payloads in a characterized fixture.
pub fn migrate_registry(
    db: &Db,
    ledger_id: LedgerIdentity,
) -> Result<EventCursorRegistry, StorageError> {
    EventCursorRegistry::migrate_legacy_payloads(db, ledger_id)
}

/// Binds a legacy cursor fixture.
pub fn legacy_cursor(tree: Tree, name: impl AsRef<str>) -> EventCursor {
    EventCursor::new(tree, name)
}

/// Binds an identity-bearing cursor fixture.
pub fn bound_cursor(tree: Tree, name: impl AsRef<str>, ledger_id: LedgerIdentity) -> EventCursor {
    EventCursor::bind_compatibility(tree, name, ledger_id)
}

/// Explicitly migrates a legacy cursor fixture after proving its identity.
pub fn migrate_cursor(
    tree: Tree,
    name: impl AsRef<str>,
    ledger_id: LedgerIdentity,
) -> Result<EventCursor, StorageError> {
    EventCursor::migrate_legacy(tree, name, ledger_id)
}

/// Binds raw observability components for low-level report characterization.
pub fn observability(
    store: Arc<EventStore>,
    watermark: Arc<CommitWatermark>,
    registry: EventCursorRegistry,
    dropped: Arc<AtomicU64>,
) -> LedgerObservability {
    LedgerObservability::new(store, watermark, registry, dropped)
}

/// Fallibly binds raw observability components for corruption tests.
pub fn try_observability(
    store: Arc<EventStore>,
    watermark: Arc<CommitWatermark>,
    registry: EventCursorRegistry,
    dropped: Arc<AtomicU64>,
) -> Result<LedgerObservability, StorageError> {
    LedgerObservability::try_new(store, watermark, registry, dropped)
}
