//! Append-only event ledger storage.
//!
//! Owner: event store.
//! Inputs: sequenced records, unsequenced envelopes, and legacy session event
//! rows.
//! Outputs: atomically sequenced append results, idempotent append results,
//! session-scoped reads, and seek-based cursor reads across sessions.
//! Does not own: this module does not publish to telemetry sinks or interpret
//! producer payloads.
//!
//! Production callers open [`crate::events::EventAuthority`] and use its
//! derived capabilities. Raw store construction is available only through
//! the `test-support` feature for frozen-format characterization.

use std::io;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(test, feature = "test-support"))]
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Db, Tree,
};
use tracing::warn;

use crate::error::StorageError;
#[cfg(any(test, feature = "test-support"))]
use crate::events::LedgerIdentity;
use crate::events::{EventEnvelope, EventRecord};

// Tree names and the legacy tree are frozen on-disk formats from the spine
// era; renaming them would buy a data migration for zero functional gain.
const TREE_EVENTS: &str = "obs_events";
const TREE_SPINE_EVENTS: &str = "obs_spine_events";
const TREE_SESSION_EVENT_INDEX: &str = "obs_session_event_index";
const TREE_SPINE_META: &str = "obs_spine_meta";
const TREE_SPINE_RECORD_INDEX: &str = "obs_spine_record_index";
const EVENT_KEY_PAD: usize = 20;
const META_KEY_GLOBAL: &[u8] = b"global";
const META_KEY_RECORD_INDEX_BACKFILLED: &[u8] = b"record_index_backfilled";
const META_KEY_LEGACY_SESSIONS_MIGRATED: &[u8] = b"legacy_sessions_migrated";
const META_KEY_SESSION_INDEX_SLIMMED: &[u8] = b"session_index_slimmed";
const META_KEY_RETAINED_FROM: &[u8] = b"retained_from";
pub(crate) const META_KEY_LEDGER_IDENTITY: &[u8] = b"ledger_identity";
const META_KEY_PRODUCT_IDENTITY: &[u8] = b"product_identity";
pub(crate) const META_KEY_AUTHORITY_CUTOVER: &[u8] = b"event_authority_cutover";
const SESSION_INDEX_EMPTY_VALUE: &[u8] = &[];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SequenceMeta {
    next_seq: u64,
}

/// Append-only event ledger backed by sled.
///
/// The store owns sequence allocation, idempotency lookup, session indexes,
/// and one-time open migrations that fold legacy session rows into the ledger.
#[derive(Clone)]
pub struct EventStore {
    db: Db,
    legacy_events: Tree,
    spine_events: Tree,
    session_event_index: Tree,
    spine_meta: Tree,
    spine_record_index: Tree,
    #[cfg(test)]
    fail_next_flush: Arc<AtomicBool>,
}

/// Result of one atomic envelope append before the authority adds identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StoreAppendOutcome {
    pub(crate) seq: u64,
    pub(crate) inserted: bool,
}

impl EventStore {
    /// Resolves the persisted identity for a raw test fixture.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn compatibility_ledger_identity(&self) -> Result<LedgerIdentity, StorageError> {
        let candidate = LedgerIdentity::new();
        let raw = match self.ledger_identity_bytes()? {
            Some(raw) => raw,
            None => self.establish_ledger_identity_bytes(&candidate.encode())?,
        };
        LedgerIdentity::decode(&raw).map_err(|error| {
            StorageError::InvalidPath(format!(
                "persisted event ledger identity must contain one 16-byte UUID: {error}"
            ))
        })
    }

    /// Opens all event trees on the supplied database handle.
    ///
    /// Opening repairs sequence metadata that lags the greatest persisted
    /// record and backfills the record index once for stores written before
    /// the index existed, so reads and idempotency lookups never scan.
    pub(crate) fn new(db: Db) -> Result<Self, StorageError> {
        let legacy_events = db.open_tree(TREE_EVENTS).map_err(to_storage_io)?;
        let spine_events = db.open_tree(TREE_SPINE_EVENTS).map_err(to_storage_io)?;
        let session_event_index = db
            .open_tree(TREE_SESSION_EVENT_INDEX)
            .map_err(to_storage_io)?;
        let spine_meta = db.open_tree(TREE_SPINE_META).map_err(to_storage_io)?;
        let spine_record_index = db
            .open_tree(TREE_SPINE_RECORD_INDEX)
            .map_err(to_storage_io)?;
        let semantic_writes_disabled = spine_meta
            .get(META_KEY_AUTHORITY_CUTOVER)
            .map_err(to_storage_io)?
            .is_some();
        let store = Self {
            db,
            legacy_events,
            spine_events,
            session_event_index,
            spine_meta,
            spine_record_index,
            #[cfg(test)]
            fail_next_flush: Arc::new(AtomicBool::new(false)),
        };
        if !semantic_writes_disabled {
            store.migrate_legacy_sessions_once()?;
            store.repair_sequence_meta()?;
            store.backfill_record_index_once()?;
            store.slim_session_index_once()?;
        }
        Ok(store)
    }

    /// Opens the store behind an `Arc` for runtimes and ingestors.
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Returns the underlying sled database.
    pub(crate) fn db(&self) -> &Db {
        &self.db
    }

    /// Appends a pre-sequenced record and advances future sequence allocation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn append_event(&self, event: &EventRecord) -> Result<(), StorageError> {
        self.write_event(event)?;
        Ok(())
    }

    /// Appends a pre-sequenced record unless its idempotency key already exists.
    #[cfg(any(test, feature = "test-support"))]
    pub fn append_event_idempotent(&self, event: &EventRecord) -> Result<u64, StorageError> {
        self.ensure_semantic_writes_allowed()?;
        let Some(record_id) = event.record_id.as_deref() else {
            self.append_event(event)?;
            return Ok(event.seq);
        };

        if let Some(existing_seq) = self.lookup_record_seq(record_id)? {
            return Ok(existing_seq);
        }

        self.write_event_idempotent(event)
    }

    /// Appends an envelope, allocating its ledger sequence atomically.
    #[cfg(any(test, feature = "test-support"))]
    pub fn append_envelope(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        self.persist_envelope_write(&envelope, false)
            .map(|outcome| outcome.seq)
    }

    /// Appends an envelope unless its idempotency key already exists.
    #[cfg(any(test, feature = "test-support"))]
    pub fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        self.persist_envelope_write(&envelope, true)
            .map(|outcome| outcome.seq)
    }

    /// Appends an envelope and reports whether this call inserted it.
    pub(crate) fn append_envelope_outcome(
        &self,
        envelope: EventEnvelope,
        idempotent: bool,
    ) -> Result<StoreAppendOutcome, StorageError> {
        self.persist_envelope_write(&envelope, idempotent)
    }

    fn write_event(&self, event: &EventRecord) -> Result<(), StorageError> {
        let write = PreparedEventWrite::new(event)?;
        self.persist_event_write(write, None).map(|_| ())
    }

    #[cfg(any(test, feature = "test-support"))]
    fn write_event_idempotent(&self, event: &EventRecord) -> Result<u64, StorageError> {
        let record_id = event
            .record_id
            .as_deref()
            .map(|record_id| record_id.as_bytes().to_vec());
        let write = PreparedEventWrite::new(event)?;
        self.persist_event_write(write, record_id)
    }

    fn persist_event_write(
        &self,
        write: PreparedEventWrite,
        idempotency_key: Option<Vec<u8>>,
    ) -> Result<u64, StorageError> {
        (
            &self.spine_meta,
            &self.spine_events,
            &self.session_event_index,
            &self.spine_record_index,
        )
            .transaction(
                |(spine_meta, spine_events, session_event_index, spine_record_index)| {
                    ensure_transaction_writes_allowed(spine_meta)?;
                    if let Some(idempotency_key) = idempotency_key.clone() {
                        if let Some(raw) = spine_record_index.get(idempotency_key)? {
                            return decode_seq(&raw).map_err(to_transaction_storage);
                        }
                    }

                    let mut meta = match spine_meta.get(META_KEY_GLOBAL)? {
                        Some(raw) => serde_json::from_slice(&raw).map_err(to_transaction_data)?,
                        None => SequenceMeta { next_seq: 1 },
                    };
                    if meta.next_seq <= write.seq {
                        meta.next_seq = write.seq + 1;
                        spine_meta.insert(
                            META_KEY_GLOBAL,
                            serde_json::to_vec(&meta).map_err(to_transaction_data)?,
                        )?;
                    }

                    spine_events.insert(write.event_key.clone(), write.value.clone())?;
                    // The sequence lives in the index key; the value stays
                    // empty so records are stored once, in the ledger tree.
                    session_event_index
                        .insert(write.session_index_key.clone(), SESSION_INDEX_EMPTY_VALUE)?;
                    if let Some((record_key, record_value)) = write.record_index.clone() {
                        spine_record_index.insert(record_key, record_value)?;
                    }

                    Ok(write.seq)
                },
            )
            .map_err(to_transaction)
    }

    /// Allocates a sequence and persists an unsequenced envelope in one
    /// serializable transaction, so concurrent appenders can never collide on
    /// a sequence or overwrite each other's records.
    fn persist_envelope_write(
        &self,
        envelope: &EventEnvelope,
        idempotent: bool,
    ) -> Result<StoreAppendOutcome, StorageError> {
        let idempotency_key = if idempotent {
            envelope
                .record_id
                .as_deref()
                .map(|record_id| record_id.as_bytes().to_vec())
        } else {
            None
        };
        (
            &self.spine_meta,
            &self.spine_events,
            &self.session_event_index,
            &self.spine_record_index,
        )
            .transaction(
                |(spine_meta, spine_events, session_event_index, spine_record_index)| {
                    ensure_transaction_writes_allowed(spine_meta)?;
                    if let Some(idempotency_key) = idempotency_key.clone() {
                        if let Some(raw) = spine_record_index.get(idempotency_key)? {
                            return decode_seq(&raw)
                                .map(|seq| StoreAppendOutcome {
                                    seq,
                                    inserted: false,
                                })
                                .map_err(to_transaction_storage);
                        }
                    }

                    let mut meta = match spine_meta.get(META_KEY_GLOBAL)? {
                        Some(raw) => serde_json::from_slice(&raw).map_err(to_transaction_data)?,
                        None => SequenceMeta { next_seq: 1 },
                    };
                    let seq = meta.next_seq;
                    meta.next_seq += 1;
                    spine_meta.insert(
                        META_KEY_GLOBAL,
                        serde_json::to_vec(&meta).map_err(to_transaction_data)?,
                    )?;

                    let record = EventRecord::from_envelope(envelope.clone(), seq);
                    let value = serde_json::to_vec(&record).map_err(to_transaction_data)?;
                    spine_events.insert(encode_record_key(seq).into_bytes(), value)?;
                    // The sequence lives in the index key; the value stays
                    // empty so records are stored once, in the ledger tree.
                    session_event_index.insert(
                        encode_session_event_index_key(&envelope.session, seq).into_bytes(),
                        SESSION_INDEX_EMPTY_VALUE,
                    )?;
                    if let Some(record_id) = envelope.record_id.as_deref() {
                        spine_record_index.insert(record_id.as_bytes(), &encode_seq(seq))?;
                    }

                    Ok(StoreAppendOutcome {
                        seq,
                        inserted: true,
                    })
                },
            )
            .map_err(to_transaction)
    }

    /// Reads all events for a session in sequence order.
    #[cfg(any(test, feature = "test-support"))]
    pub fn read_events(&self, session_id: &str) -> Result<Vec<EventRecord>, StorageError> {
        self.read_events_after(session_id, 0)
    }

    /// Reads events for a session after a ledger sequence.
    #[cfg(any(test, feature = "test-support"))]
    pub fn read_events_after(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<Vec<EventRecord>, StorageError> {
        self.read_session_events_after(session_id, after_seq)
    }

    /// Reads all events after a ledger sequence across sessions.
    ///
    /// Record keys are zero-padded sequences, so the read seeks directly to
    /// the cursor and returns records in sequence order; cost is proportional
    /// to the records returned, not to total history.
    #[cfg(any(test, feature = "test-support"))]
    pub fn read_all_events_after(&self, after_seq: u64) -> Result<Vec<EventRecord>, StorageError> {
        self.check_retention(after_seq)?;
        let mut out = Vec::new();
        let start = encode_record_key(after_seq.saturating_add(1)).into_bytes();
        for result in self.spine_events.range(start..) {
            let (_, value) = result.map_err(to_storage_io)?;
            out.push(decode_event(&value)?);
        }
        Ok(out)
    }

    /// Reads at most `limit` events after a ledger sequence across sessions,
    /// in sequence order.
    #[cfg(any(test, feature = "test-support"))]
    pub fn read_all_events_after_limit(
        &self,
        after_seq: u64,
        limit: usize,
    ) -> Result<Vec<EventRecord>, StorageError> {
        self.check_retention(after_seq)?;
        if limit == 0 {
            return Ok(Vec::new());
        }

        // The caller-provided limit is not trusted at this layer. Grow only
        // as records are actually decoded instead of reserving the full
        // requested capacity up front.
        let mut out = Vec::new();
        let start = encode_record_key(after_seq.saturating_add(1)).into_bytes();
        for result in self.spine_events.range(start..).take(limit) {
            let (_, value) = result.map_err(to_storage_io)?;
            out.push(decode_event(&value)?);
        }
        Ok(out)
    }

    /// Reads at most `limit` events after a cursor and no later than a frozen
    /// ledger tip.
    pub(crate) fn read_all_events_between_limit(
        &self,
        after_seq: u64,
        through_seq: u64,
        limit: usize,
    ) -> Result<Vec<EventRecord>, StorageError> {
        self.check_retention(after_seq)?;
        if limit == 0 || after_seq >= through_seq {
            return Ok(Vec::new());
        }

        let start = encode_record_key(after_seq.saturating_add(1)).into_bytes();
        let end = encode_record_key(through_seq).into_bytes();
        let mut out = Vec::new();
        for result in self.spine_events.range(start..=end).take(limit) {
            let (_, value) = result.map_err(to_storage_io)?;
            out.push(decode_event(&value)?);
        }
        Ok(out)
    }

    /// Resolves one exact durable sequence for an authority-owned append proof.
    pub(crate) fn event_at(&self, seq: u64) -> Result<Option<EventRecord>, StorageError> {
        self.spine_events
            .get(encode_record_key(seq).as_bytes())
            .map_err(to_storage_io)?
            .map(|raw| decode_event(&raw))
            .transpose()
    }

    /// Reads at most `limit` retained events ending at `through_seq`.
    ///
    /// Selection is by record count rather than sequence distance, so sparse
    /// imported ledgers still return the newest requested number of records.
    /// Results are restored to ascending sequence order for consumers.
    pub(crate) fn read_newest_events_through(
        &self,
        through_seq: u64,
        limit: usize,
    ) -> Result<Vec<EventRecord>, StorageError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let retained_from = self.retained_lower_boundary()?;
        if through_seq < retained_from {
            return Ok(Vec::new());
        }
        let start = encode_record_key(retained_from).into_bytes();
        let end = encode_record_key(through_seq).into_bytes();
        let mut out = Vec::new();
        for result in self.spine_events.range(start..=end).rev().take(limit) {
            let (_, value) = result.map_err(to_storage_io)?;
            out.push(decode_event(&value)?);
        }
        out.reverse();
        Ok(out)
    }

    /// Returns the highest persisted ledger sequence, zero when empty.
    ///
    /// Derived from the zero-padded key so one undecodable record cannot
    /// fail the read.
    pub fn tip_seq(&self) -> Result<u64, StorageError> {
        let Some((key, _)) = self.spine_events.last().map_err(to_storage_io)? else {
            return Ok(0);
        };
        Ok(std::str::from_utf8(&key)
            .ok()
            .and_then(|key| key.parse::<u64>().ok())
            .unwrap_or(0))
    }

    /// Returns the first sequence still retained by the ledger.
    ///
    /// One means full history. A future compactor raises the boundary when
    /// it prunes; until then it never moves.
    pub fn retained_lower_boundary(&self) -> Result<u64, StorageError> {
        let Some(raw) = self
            .spine_meta
            .get(META_KEY_RETAINED_FROM)
            .map_err(to_storage_io)?
        else {
            return Ok(1);
        };
        decode_seq(&raw)
    }

    /// Raises the retained lower boundary; the compactor's contract hook.
    ///
    /// Raising the boundary promises that every sequence below it is gone;
    /// replay from a cursor below the boundary returns a typed retention gap
    /// instead of silently skipping history. The boundary never lowers.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_retained_lower_boundary(&self, retained_from: u64) -> Result<(), StorageError> {
        self.ensure_semantic_writes_allowed()?;
        let current = self.retained_lower_boundary()?;
        if retained_from <= current {
            return Ok(());
        }
        self.spine_meta
            .insert(META_KEY_RETAINED_FROM, &encode_seq(retained_from))
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Fails with a typed retention gap when a cursor predates retained
    /// history, so no replay can silently skip compacted events.
    fn check_retention(&self, after_seq: u64) -> Result<(), StorageError> {
        let retained_from = self.retained_lower_boundary()?;
        if after_seq.saturating_add(1) < retained_from {
            return Err(StorageError::RetentionGap {
                after_seq,
                retained_from,
            });
        }
        Ok(())
    }

    /// Flushes pending sled writes to durable storage.
    pub fn flush(&self) -> Result<(), StorageError> {
        #[cfg(test)]
        if self.fail_next_flush.swap(false, Ordering::SeqCst) {
            return Err(StorageError::IoError(io::Error::other(
                "injected event-store flush failure",
            )));
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fail_next_flush_for_test(&self) {
        self.fail_next_flush.store(true, Ordering::SeqCst);
    }

    /// Reads the raw persisted ledger identity for authority validation.
    pub(crate) fn ledger_identity_bytes(&self) -> Result<Option<Vec<u8>>, StorageError> {
        self.spine_meta
            .get(META_KEY_LEDGER_IDENTITY)
            .map(|value| value.map(|bytes| bytes.to_vec()))
            .map_err(to_storage_io)
    }

    /// Atomically establishes a ledger identity or returns the concurrent
    /// winner, flushing a newly inserted value before returning.
    pub(crate) fn establish_ledger_identity_bytes(
        &self,
        identity: &[u8],
    ) -> Result<Vec<u8>, StorageError> {
        self.ensure_semantic_writes_allowed()?;
        match self
            .spine_meta
            .compare_and_swap(
                META_KEY_LEDGER_IDENTITY,
                None as Option<&[u8]>,
                Some(identity),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => {
                self.flush()?;
                Ok(identity.to_vec())
            }
            Err(conflict) => {
                let current = conflict
                    .current
                    .map(|value| value.to_vec())
                    .ok_or_else(|| {
                        StorageError::IoError(io::Error::other(
                            "ledger identity compare-and-swap lost without a winner",
                        ))
                    })?;
                self.flush()?;
                Ok(current)
            }
        }
    }

    /// Atomically binds this ledger to one product identity.
    pub(crate) fn bind_product_identity(&self, product_identity: &str) -> Result<(), StorageError> {
        if product_identity.is_empty() {
            return Err(StorageError::InvalidPath(
                "product identity must not be empty".to_string(),
            ));
        }
        let requested = product_identity.as_bytes();
        match self
            .spine_meta
            .compare_and_swap(
                META_KEY_PRODUCT_IDENTITY,
                None as Option<&[u8]>,
                Some(requested),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => self.flush(),
            Err(conflict) => {
                let current = conflict.current.ok_or_else(|| {
                    StorageError::IoError(io::Error::other(
                        "product identity compare-and-swap lost without a winner",
                    ))
                })?;
                let current = std::str::from_utf8(&current).map_err(|error| {
                    StorageError::MigrationConflict(format!(
                        "persisted product identity is not UTF-8: {error}"
                    ))
                })?;
                if current == product_identity {
                    self.flush()
                } else {
                    Err(StorageError::MigrationConflict(format!(
                        "event ledger is bound to product {current}, not {product_identity}"
                    )))
                }
            }
        }
    }

    /// Encodes a legacy session event key.
    #[cfg(any(test, feature = "test-support"))]
    pub fn encode_event_key(session_id: &str, seq: u64) -> String {
        encode_legacy_event_key(session_id, seq)
    }

    /// Seeks within one session's key range and resolves each entry through
    /// the ledger tree. Session keys are `{session}:{seq:020}`, so the range
    /// starts at the cursor, `;` bounds the `:` separator, and the sequence
    /// comes from the key tail; index values carry no record payload.
    #[cfg(any(test, feature = "test-support"))]
    fn read_session_events_after(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<Vec<EventRecord>, StorageError> {
        self.check_retention(after_seq)?;
        let start =
            encode_session_event_index_key(session_id, after_seq.saturating_add(1)).into_bytes();
        let end = format!("{session_id};").into_bytes();
        let mut out = Vec::new();
        for result in self.session_event_index.range(start..end) {
            let (key, _) = result.map_err(to_storage_io)?;
            let Some(seq) = decode_session_key_seq(&key) else {
                warn!(
                    key = %String::from_utf8_lossy(&key),
                    "skipping malformed session index key"
                );
                continue;
            };
            // Foreign session ids sharing this prefix plus a colon byte-sort
            // into the range regardless of their sequence; the filter keeps
            // exact cursor semantics for those leaked index entries.
            if seq <= after_seq {
                continue;
            }
            let Some(value) = self
                .spine_events
                .get(encode_record_key(seq).as_bytes())
                .map_err(to_storage_io)?
            else {
                warn!(seq, "session index entry has no ledger record");
                continue;
            };
            let record = decode_event(&value)?;
            if record.seq != seq {
                warn!(
                    index_seq = seq,
                    record_seq = record.seq,
                    "session index sequence disagrees with ledger record"
                );
                continue;
            }
            if record.session != session_id {
                warn!(
                    index_seq = seq,
                    requested_session = session_id,
                    record_session = record.session,
                    "session index entry resolves to a foreign session"
                );
                continue;
            }
            out.push(record);
        }
        Ok(out)
    }

    #[cfg(any(test, feature = "test-support"))]
    fn lookup_record_seq(&self, record_id: &str) -> Result<Option<u64>, StorageError> {
        let Some(raw) = self
            .spine_record_index
            .get(record_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(decode_seq(&raw)?))
    }

    fn ensure_semantic_writes_allowed(&self) -> Result<(), StorageError> {
        if self
            .spine_meta
            .get(META_KEY_AUTHORITY_CUTOVER)
            .map_err(to_storage_io)?
            .is_some()
        {
            Err(StorageError::MigrationConflict(
                "legacy event ledger is read-only after authority cutover".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Repairs sequence metadata that lags the greatest persisted record, so
    /// allocation can never reuse a persisted sequence after legacy writes or
    /// external tampering left metadata behind.
    fn repair_sequence_meta(&self) -> Result<(), StorageError> {
        let Some((key, _)) = self.spine_events.last().map_err(to_storage_io)? else {
            return Ok(());
        };
        // The sequence is derived from the zero-padded key rather than the
        // record value, so one undecodable record cannot fail store open.
        let Some(max_seq) = std::str::from_utf8(&key)
            .ok()
            .and_then(|key| key.parse::<u64>().ok())
        else {
            warn!("event tree tail key is not a sequence; skipping meta repair");
            return Ok(());
        };
        let meta = match self
            .spine_meta
            .get(META_KEY_GLOBAL)
            .map_err(to_storage_io)?
        {
            Some(raw) => serde_json::from_slice::<SequenceMeta>(&raw).map_err(to_storage_data)?,
            None => SequenceMeta { next_seq: 1 },
        };
        if meta.next_seq <= max_seq {
            let repaired = SequenceMeta {
                next_seq: max_seq + 1,
            };
            let value = serde_json::to_vec(&repaired).map_err(to_storage_data)?;
            self.spine_meta
                .insert(META_KEY_GLOBAL, value)
                .map_err(to_storage_io)?;
        }
        Ok(())
    }

    // TODO compat-shim(post-E5): remove once no supported deployed store predates the ledger
    // trees, together with `encode_event_key`. Migrates rows from the legacy
    // session event tree into the ledger so session reads have one source;
    // before this, reads merged the legacy tree on every call. Removal
    // requires multi_session_legacy_stores_migrate_completely and
    // legacy_rows_coexist_with_ledger_history_after_migration to stay green
    // against a store created without legacy rows.
    fn migrate_legacy_sessions_once(&self) -> Result<(), StorageError> {
        if self
            .spine_meta
            .get(META_KEY_LEGACY_SESSIONS_MIGRATED)
            .map_err(to_storage_io)?
            .is_some()
        {
            return Ok(());
        }

        // Legacy sequences were allocated per session and restart at one in
        // every session, so they cannot map onto the global ledger keyspace.
        // Every row is re-sequenced through fresh global allocation instead;
        // lexicographic key order groups sessions and preserves each
        // session's relative order. Rows are decoded outside the transaction
        // so the closure stays pure over this list on retry.
        let mut rows = Vec::new();
        for result in self.legacy_events.iter() {
            let (key, value) = result.map_err(to_storage_io)?;
            // An undecodable legacy row cannot be served by any read path;
            // skipped with a warning instead of failing store open.
            match decode_event(&value) {
                Ok(event) => rows.push(event),
                Err(error) => {
                    warn!(
                        key = %String::from_utf8_lossy(&key),
                        error = %error,
                        "skipping undecodable legacy row during migration"
                    );
                }
            }
        }

        // One transaction covers every migrated row, the sequence metadata,
        // and the flag, so a crash can never leave partial migration state
        // or a durable flag over missing history.
        (
            &self.spine_meta,
            &self.spine_events,
            &self.session_event_index,
        )
            .transaction(|(spine_meta, spine_events, session_event_index)| {
                let mut meta = match spine_meta.get(META_KEY_GLOBAL)? {
                    Some(raw) => serde_json::from_slice(&raw).map_err(to_transaction_data)?,
                    None => SequenceMeta { next_seq: 1 },
                };
                for event in &rows {
                    let seq = meta.next_seq;
                    meta.next_seq += 1;
                    let record = EventRecord::from_envelope(event.envelope.clone(), seq);
                    let value = serde_json::to_vec(&record).map_err(to_transaction_data)?;
                    spine_events.insert(encode_record_key(seq).into_bytes(), value)?;
                    session_event_index.insert(
                        encode_session_event_index_key(&record.session, seq).into_bytes(),
                        SESSION_INDEX_EMPTY_VALUE,
                    )?;
                }
                spine_meta.insert(
                    META_KEY_GLOBAL,
                    serde_json::to_vec(&meta).map_err(to_transaction_data)?,
                )?;
                spine_meta.insert(META_KEY_LEGACY_SESSIONS_MIGRATED, &[1u8])?;
                Ok(())
            })
            .map_err(to_transaction)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    // TODO compat-shim(post-E5): remove once no supported deployed store predates empty session
    // index values. Rewrites full-record index values to empty markers so the
    // ledger tree is the only copy of each record. Removal requires the
    // full_value_session_index_rows_slim_at_open to stay green against a
    // store created without full-value rows.
    fn slim_session_index_once(&self) -> Result<(), StorageError> {
        if self
            .spine_meta
            .get(META_KEY_SESSION_INDEX_SLIMMED)
            .map_err(to_storage_io)?
            .is_some()
        {
            return Ok(());
        }
        for result in self.session_event_index.iter() {
            let (key, value) = result.map_err(to_storage_io)?;
            if !value.is_empty() {
                self.session_event_index
                    .insert(key, SESSION_INDEX_EMPTY_VALUE)
                    .map_err(to_storage_io)?;
            }
        }
        self.spine_meta
            .insert(META_KEY_SESSION_INDEX_SLIMMED, &[1u8])
            .map_err(to_storage_io)?;
        Ok(())
    }

    // TODO compat-shim(post-E5): remove once no supported deployed store predates the record
    // index. Backfills index entries for records persisted before the index
    // tree existed, replacing the old per-lookup full-tree scan fallback.
    // Removal requires idempotent_append_reuses_record_sequence_and_survives_reopen
    // and store_open_repairs_missing_record_index_and_sequence_meta to stay
    // green against a store created without this backfill.
    fn backfill_record_index_once(&self) -> Result<(), StorageError> {
        if self
            .spine_meta
            .get(META_KEY_RECORD_INDEX_BACKFILLED)
            .map_err(to_storage_io)?
            .is_some()
        {
            return Ok(());
        }
        for result in self.spine_events.iter() {
            let (key, value) = result.map_err(to_storage_io)?;
            // An undecodable record cannot be idempotency-replayed, so it is
            // skipped with a warning instead of failing store open.
            let event = match decode_event(&value) {
                Ok(event) => event,
                Err(error) => {
                    warn!(
                        key = %String::from_utf8_lossy(&key),
                        error = %error,
                        "skipping undecodable record during index backfill"
                    );
                    continue;
                }
            };
            let Some(record_id) = event.record_id.as_deref() else {
                continue;
            };
            if self
                .spine_record_index
                .get(record_id.as_bytes())
                .map_err(to_storage_io)?
                .is_none()
            {
                // Full rewrite rather than a bare index insert: pre-index
                // records may also miss their session index entry, and the
                // old per-lookup repair restored both.
                self.write_event(&event)?;
            }
        }
        // The flag must not become durable before the repairs it records:
        // with the per-lookup scan fallback gone, a persisted flag over lost
        // repairs would duplicate idempotent replays forever.
        self.db.flush().map_err(to_storage_io)?;
        self.spine_meta
            .insert(META_KEY_RECORD_INDEX_BACKFILLED, &[1u8])
            .map_err(to_storage_io)?;
        Ok(())
    }
}

fn ensure_transaction_writes_allowed(
    spine_meta: &sled::transaction::TransactionalTree,
) -> Result<(), ConflictableTransactionError<StorageError>> {
    if spine_meta.get(META_KEY_AUTHORITY_CUTOVER)?.is_some() {
        Err(ConflictableTransactionError::Abort(
            StorageError::MigrationConflict(
                "legacy event ledger is read-only after authority cutover".to_string(),
            ),
        ))
    } else {
        Ok(())
    }
}

struct PreparedEventWrite {
    seq: u64,
    event_key: Vec<u8>,
    session_index_key: Vec<u8>,
    value: Vec<u8>,
    record_index: Option<(Vec<u8>, Vec<u8>)>,
}

impl PreparedEventWrite {
    fn new(event: &EventRecord) -> Result<Self, StorageError> {
        Ok(Self {
            seq: event.seq,
            event_key: encode_record_key(event.seq).into_bytes(),
            session_index_key: encode_session_event_index_key(&event.session, event.seq)
                .into_bytes(),
            value: serde_json::to_vec(event).map_err(to_storage_data)?,
            record_index: event.record_id.as_deref().map(|record_id| {
                (
                    record_id.as_bytes().to_vec(),
                    encode_seq(event.seq).to_vec(),
                )
            }),
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
fn encode_legacy_event_key(session_id: &str, seq: u64) -> String {
    format!("{session_id}:{seq:0EVENT_KEY_PAD$}")
}

fn encode_record_key(seq: u64) -> String {
    format!("{seq:0EVENT_KEY_PAD$}")
}

/// Parses the sequence from a session index key's fixed-width tail.
#[cfg(any(test, feature = "test-support"))]
fn decode_session_key_seq(key: &[u8]) -> Option<u64> {
    if key.len() <= EVENT_KEY_PAD {
        return None;
    }
    std::str::from_utf8(&key[key.len() - EVENT_KEY_PAD..])
        .ok()?
        .parse()
        .ok()
}

fn encode_session_event_index_key(session_id: &str, seq: u64) -> String {
    format!("{session_id}:{seq:0EVENT_KEY_PAD$}")
}

fn encode_seq(seq: u64) -> [u8; 8] {
    seq.to_be_bytes()
}

fn decode_seq(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::IoError(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid ledger record index payload",
        ))
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_event(raw: &[u8]) -> Result<EventRecord, StorageError> {
    Ok(serde_json::from_slice::<EventRecord>(raw)
        .map_err(to_storage_data)?
        .normalize_legacy_defaults())
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_transaction_data(err: serde_json::Error) -> ConflictableTransactionError<StorageError> {
    ConflictableTransactionError::Abort(to_storage_data(err))
}

fn to_transaction_storage(err: StorageError) -> ConflictableTransactionError<StorageError> {
    ConflictableTransactionError::Abort(err)
}

fn to_transaction(err: TransactionError<StorageError>) -> StorageError {
    match err {
        TransactionError::Abort(err) => err,
        TransactionError::Storage(err) => to_storage_io(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record(seq: u64, ts: &str, session: &str, event_type: &str) -> EventRecord {
        EventRecord::from_envelope(
            EventEnvelope::new(
                ts.to_string(),
                session.to_string(),
                event_type,
                serde_json::json!({}),
            ),
            seq,
        )
    }

    #[test]
    fn key_encoding_is_lexicographic() {
        let k1 = EventStore::encode_event_key("s1", 2);
        let k2 = EventStore::encode_event_key("s1", 10);
        assert!(k1 < k2);
    }

    #[test]
    fn write_and_read_events_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::new(db).unwrap();
        let session = "abc";

        let e2 = test_record(2, "2", session, "session_ended");
        let e1 = test_record(1, "1", session, "session_started");
        store.append_event(&e2).unwrap();
        store.append_event(&e1).unwrap();
        let events = store.read_events(session).unwrap();
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
    }

    #[test]
    fn read_all_events_after_returns_runtime_order() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::new(db).unwrap();

        let e1 = test_record(1, "1", "s1", "session_started");
        let e2 = test_record(2, "2", "s2", "session_started");

        store.append_event(&e2).unwrap();
        store.append_event(&e1).unwrap();

        let events = store.read_all_events_after(0).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
    }

    // Legacy rows persisted before the ledger trees existed are migrated
    // into the ledger at open, after which reads never consult the legacy
    // tree; a row that appears there later is intentionally invisible.
    #[test]
    fn legacy_events_migrate_into_ledger_at_open() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let session = "legacy_session";

        let legacy_tree = db.open_tree("obs_events").unwrap();
        let key = EventStore::encode_event_key(session, 1);
        let raw = serde_json::to_vec(&serde_json::json!({
            "ts": "1",
            "recorded_at": "",
            "record_id": null,
            "session": session,
            "seq": 1,
            "domain_id": "",
            "stream_id": "",
            "type": "session_started",
            "occurred_at": null,
            "content_hash": null,
            "objects": [],
            "relations": [],
            "data": {}
        }))
        .unwrap();
        legacy_tree.insert(key.as_bytes(), raw).unwrap();

        let store = EventStore::new(db).unwrap();
        let events = store.read_events(session).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].domain_id, "telemetry");
        assert_eq!(events[0].stream_id, session);
        assert_eq!(store.read_all_events_after(0).unwrap().len(), 1);
        // Migration advanced sequence metadata past the migrated row.
        assert_eq!(
            store
                .append_envelope(EventEnvelope::new(
                    "2".to_string(),
                    session.to_string(),
                    "session_ended",
                    serde_json::json!({}),
                ))
                .unwrap(),
            2
        );
    }
}
