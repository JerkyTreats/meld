//! Identity-bearing aggregate for all canonical event capabilities.
//!
//! Owner: event authority.
//! Inputs: one sled database and optional expected ledger identity.
//! Outputs: append, replay, subscription, watermark, consumer-registry, and
//! observability capabilities sharing one writer, store, and identity.
//! Does not own: product binding, daemon hosting, transport, or scheduling.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::EventAuthorityError;
use crate::events::identity::LedgerIdentity;
use crate::events::observability::{CoverageTruncation, EventReadCoverage};
use crate::events::registry::{ConsumerCursor, EventCursorRegistry};
use crate::events::store::EventStore;
use crate::events::writer::EventWriter;
use crate::events::{EventEnvelope, EventRecord};

/// Largest replay page accepted by the authority.
pub const MAX_REPLAY_LIMIT: usize = 1_024;
/// Largest subscription wait accepted by the authority, in milliseconds.
pub const MAX_SUBSCRIPTION_TIMEOUT_MS: u64 = 30_000;

/// Options controlling identity validation while opening an authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventAuthorityOpenOptions {
    /// Required identity for a product-bound ledger, when already known.
    pub expected_ledger_id: Option<LedgerIdentity>,
}

/// Canonical owner of one writable event ledger in this process.
#[derive(Clone)]
pub struct EventAuthority {
    inner: Arc<AuthorityInner>,
}

struct AuthorityInner {
    ledger_id: LedgerIdentity,
    store: Arc<EventStore>,
    writer: EventWriter,
    registry: EventCursorRegistry,
    _lease: AuthorityLease,
}

/// Durability and deduplication behavior for one append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppendMode {
    /// Always allocate and append a new record.
    Plain,
    /// Reuse the first sequence for an envelope's `record_id`.
    Idempotent,
}

/// Whether an acknowledged durable append created a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppendDisposition {
    /// This append inserted a new canonical record.
    Inserted,
    /// An idempotent append found the existing canonical record.
    Duplicate,
}

/// Durable, identity-bearing append acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppendReceipt {
    /// Ledger that owns the returned sequence.
    pub ledger_id: LedgerIdentity,
    /// Canonical ledger sequence.
    pub seq: u64,
    /// Whether this call inserted the record.
    pub disposition: AppendDisposition,
}

/// Accepted best-effort append without a durability or sequence claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BestEffortAppendReceipt {
    /// Ledger whose ingress queue accepted the envelope.
    pub ledger_id: LedgerIdentity,
}

/// Identity-bearing position immediately after a ledger sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerCursor {
    /// Ledger whose sequence space the cursor addresses.
    pub ledger_id: LedgerIdentity,
    /// Highest sequence already consumed.
    pub after_seq: u64,
}

/// One bounded replay request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayRequest {
    /// Starting cursor, exclusive.
    pub cursor: LedgerCursor,
    /// Maximum records returned, from one through [`MAX_REPLAY_LIMIT`].
    pub limit: usize,
}

/// One bounded replay page with honest durable coverage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPage {
    /// Ledger that owns every returned sequence.
    pub ledger_id: LedgerIdentity,
    /// Canonical records in ascending sequence order.
    pub records: Vec<EventRecord>,
    /// Cursor to use for the next request.
    pub next_cursor: LedgerCursor,
    /// Frozen durable range inspected by this replay.
    pub coverage: EventReadCoverage,
}

/// Blocking subscription request built on bounded replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPollRequest {
    /// Replay request attempted before and after any wait.
    pub replay: ReplayRequest,
    /// Maximum wait in milliseconds, from zero through 30,000.
    pub timeout_ms: u64,
}

/// Identity-bearing watermark and durable-tip snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventWatermark {
    /// Ledger whose sequences are reported.
    pub ledger_id: LedgerIdentity,
    /// Highest sequence durably committed through the authority writer.
    pub committed_seq: u64,
    /// Highest sequence currently durable in the ledger.
    pub tip_seq: u64,
}

/// Identity-bearing consumer registry row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerCursorPosition {
    /// Ledger whose sequence space the position addresses.
    pub ledger_id: LedgerIdentity,
    /// Stable consumer name.
    pub name: String,
    /// Highest sequence reported durable by the consumer.
    pub reported_seq: u64,
}

/// Clonable durable and best-effort append capability.
#[derive(Clone)]
pub struct EventAppendCapability {
    inner: Arc<AuthorityInner>,
}

/// Clonable bounded replay capability.
#[derive(Clone)]
pub struct EventReplayCapability {
    inner: Arc<AuthorityInner>,
}

/// Clonable blocking subscription capability.
#[derive(Clone)]
pub struct EventSubscriptionCapability {
    inner: Arc<AuthorityInner>,
}

/// Clonable committed-watermark capability.
#[derive(Clone)]
pub struct EventWatermarkCapability {
    inner: Arc<AuthorityInner>,
}

/// Clonable identity-bound consumer-registry capability.
#[derive(Clone)]
pub struct EventConsumerRegistryCapability {
    inner: Arc<AuthorityInner>,
}

/// Clonable observability input capability.
///
/// E3 binds the identity-bearing public report methods to this handle. Its
/// E2 public surface intentionally exposes only identity; crate-owned
/// observability computation can access the aggregate without exposing raw
/// storage to domains.
#[derive(Clone)]
pub struct EventObservabilityCapability {
    inner: Arc<AuthorityInner>,
}

impl EventAuthority {
    /// Opens and repairs storage, establishes identity, recovers the durable
    /// watermark, and acquires the process-local writable lease.
    pub fn open(
        db: sled::Db,
        options: EventAuthorityOpenOptions,
    ) -> Result<Self, EventAuthorityError> {
        let store = Arc::new(EventStore::new(db)?);
        let candidate = options.expected_ledger_id.unwrap_or_default();
        let identity_bytes = match store.ledger_identity_bytes()? {
            Some(raw) => raw,
            None => store.establish_ledger_identity_bytes(&candidate.encode())?,
        };
        let ledger_id = LedgerIdentity::decode(&identity_bytes).map_err(|error| {
            EventAuthorityError::CorruptPersistedIdentity {
                message: format!("ledger_identity must contain one 16-byte UUID: {error}"),
            }
        })?;
        if let Some(expected) = options.expected_ledger_id {
            if ledger_id != expected {
                return Err(EventAuthorityError::IdentityMismatch {
                    expected,
                    actual: ledger_id,
                });
            }
        }

        let lease = AuthorityLease::acquire(ledger_id)?;
        let registry = EventCursorRegistry::open_bound(store.db(), ledger_id)?;
        let durable_tip = store.tip_seq()?;
        let writer = EventWriter::spawn_recovered(Arc::clone(&store), durable_tip);
        Ok(Self {
            inner: Arc::new(AuthorityInner {
                ledger_id,
                store,
                writer,
                registry,
                _lease: lease,
            }),
        })
    }

    /// Returns this authority's durable ledger identity.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Derives the shared append capability.
    pub fn append_capability(&self) -> EventAppendCapability {
        EventAppendCapability {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Derives the shared replay capability.
    pub fn replay_capability(&self) -> EventReplayCapability {
        EventReplayCapability {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Derives the shared subscription capability.
    pub fn subscription_capability(&self) -> EventSubscriptionCapability {
        EventSubscriptionCapability {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Derives the shared watermark capability.
    pub fn watermark_capability(&self) -> EventWatermarkCapability {
        EventWatermarkCapability {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Derives the shared consumer-registry capability.
    pub fn consumer_registry_capability(&self) -> EventConsumerRegistryCapability {
        EventConsumerRegistryCapability {
            inner: Arc::clone(&self.inner),
        }
    }

    /// Derives the shared observability capability.
    pub fn observability_capability(&self) -> EventObservabilityCapability {
        EventObservabilityCapability {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl EventAppendCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Appends and flushes one envelope before returning its receipt.
    pub fn append_durable(
        &self,
        envelope: EventEnvelope,
        mode: AppendMode,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        let outcome = self
            .inner
            .writer
            .append_durable_outcome(envelope, mode == AppendMode::Idempotent)?;
        Ok(AppendReceipt {
            ledger_id: self.inner.ledger_id,
            seq: outcome.seq,
            disposition: if outcome.inserted {
                AppendDisposition::Inserted
            } else {
                AppendDisposition::Duplicate
            },
        })
    }

    /// Appends and flushes a batch through the same group commit, preserving
    /// one identity-bearing receipt per input envelope.
    pub fn append_durable_batch(
        &self,
        envelopes: Vec<EventEnvelope>,
        mode: AppendMode,
    ) -> Result<Vec<AppendReceipt>, EventAuthorityError> {
        self.inner
            .writer
            .append_durable_outcomes_batch(envelopes, mode == AppendMode::Idempotent)?
            .into_iter()
            .map(|outcome| {
                Ok(AppendReceipt {
                    ledger_id: self.inner.ledger_id,
                    seq: outcome.seq,
                    disposition: if outcome.inserted {
                        AppendDisposition::Inserted
                    } else {
                        AppendDisposition::Duplicate
                    },
                })
            })
            .collect()
    }

    /// Enqueues an envelope without claiming durability or a sequence.
    pub fn append_best_effort(
        &self,
        envelope: EventEnvelope,
        mode: AppendMode,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError> {
        self.inner
            .writer
            .enqueue_best_effort(envelope, mode == AppendMode::Idempotent)?;
        Ok(BestEffortAppendReceipt {
            ledger_id: self.inner.ledger_id,
        })
    }

    /// Waits until every previously accepted best-effort append has been
    /// processed and its group flush attempted.
    pub fn barrier(&self) -> Result<(), EventAuthorityError> {
        self.inner.writer.barrier().map_err(Into::into)
    }
}

impl EventReplayCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Replays one identity-checked, bounded page at a frozen durable tip.
    pub fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        replay(&self.inner, request)
    }
}

impl EventSubscriptionCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Returns a replay page immediately or waits for the authority watermark
    /// before replaying once more.
    pub fn poll(&self, request: SubscriptionPollRequest) -> Result<EventPage, EventAuthorityError> {
        validate_identity(self.inner.ledger_id, request.replay.cursor.ledger_id)?;
        if request.timeout_ms > MAX_SUBSCRIPTION_TIMEOUT_MS {
            return Err(EventAuthorityError::invalid_request(format!(
                "subscription timeout_ms must be in 0..={MAX_SUBSCRIPTION_TIMEOUT_MS}, got {}",
                request.timeout_ms
            )));
        }
        let first = replay(&self.inner, request.replay)?;
        if !first.records.is_empty() || request.timeout_ms == 0 {
            return Ok(first);
        }
        self.inner.writer.watermark().wait_past(
            request.replay.cursor.after_seq,
            Duration::from_millis(request.timeout_ms),
        );
        replay(&self.inner, request.replay)
    }
}

impl EventWatermarkCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Returns the recovered committed watermark and current durable tip.
    pub fn snapshot(&self) -> Result<EventWatermark, EventAuthorityError> {
        Ok(EventWatermark {
            ledger_id: self.inner.ledger_id,
            committed_seq: self.inner.writer.watermark().committed_seq(),
            tip_seq: self.inner.store.tip_seq()?,
        })
    }

    /// Waits for the committed watermark to pass an identity-checked cursor.
    pub fn wait_past(
        &self,
        cursor: LedgerCursor,
        timeout: Duration,
    ) -> Result<EventWatermark, EventAuthorityError> {
        validate_identity(self.inner.ledger_id, cursor.ledger_id)?;
        self.inner
            .writer
            .watermark()
            .wait_past(cursor.after_seq, timeout);
        self.snapshot()
    }
}

impl EventConsumerRegistryCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }

    /// Reports a durable consumer cursor monotonically.
    pub fn report(
        &self,
        name: &str,
        cursor: LedgerCursor,
    ) -> Result<ConsumerCursorPosition, EventAuthorityError> {
        validate_identity(self.inner.ledger_id, cursor.ledger_id)?;
        let reported_seq = self.inner.registry.report(name, cursor.after_seq)?;
        Ok(ConsumerCursorPosition {
            ledger_id: self.inner.ledger_id,
            name: name.to_string(),
            reported_seq,
        })
    }

    /// Returns one registered durable cursor.
    pub fn get(&self, name: &str) -> Result<Option<ConsumerCursorPosition>, EventAuthorityError> {
        Ok(self
            .inner
            .registry
            .get(name)?
            .map(|reported_seq| ConsumerCursorPosition {
                ledger_id: self.inner.ledger_id,
                name: name.to_string(),
                reported_seq,
            }))
    }

    /// Returns every registered durable cursor in name order.
    pub fn snapshot(&self) -> Result<Vec<ConsumerCursorPosition>, EventAuthorityError> {
        self.inner
            .registry
            .snapshot()?
            .into_iter()
            .map(|cursor| Ok(self.position(cursor)))
            .collect()
    }

    fn position(&self, cursor: ConsumerCursor) -> ConsumerCursorPosition {
        ConsumerCursorPosition {
            ledger_id: self.inner.ledger_id,
            name: cursor.name,
            reported_seq: cursor.reported_seq,
        }
    }
}

impl EventObservabilityCapability {
    /// Returns the ledger accepted by this capability.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.inner.ledger_id
    }
}

fn replay(
    inner: &AuthorityInner,
    request: ReplayRequest,
) -> Result<EventPage, EventAuthorityError> {
    validate_identity(inner.ledger_id, request.cursor.ledger_id)?;
    if !(1..=MAX_REPLAY_LIMIT).contains(&request.limit) {
        return Err(EventAuthorityError::invalid_request(format!(
            "replay limit must be in 1..={MAX_REPLAY_LIMIT}, got {}",
            request.limit
        )));
    }

    let tip_seq = inner.store.tip_seq()?;
    let retained_from = inner.store.retained_lower_boundary()?;
    let mut records = inner.store.read_all_events_between_limit(
        request.cursor.after_seq,
        tip_seq,
        request.limit + 1,
    )?;
    let truncated_after = records.len() > request.limit;
    records.truncate(request.limit);
    let scanned_from_seq = records.first().map(|record| record.seq);
    let scanned_through_seq = records.last().map(|record| record.seq);
    let truncated_before =
        retained_from > 1 || (tip_seq > 0 && request.cursor.after_seq >= retained_from);
    let truncation = match (truncated_before, truncated_after) {
        (false, false) => CoverageTruncation::None,
        (true, false) => CoverageTruncation::Before,
        (false, true) => CoverageTruncation::After,
        (true, true) => CoverageTruncation::Both,
    };
    let next_after_seq = scanned_through_seq.unwrap_or(request.cursor.after_seq);
    Ok(EventPage {
        ledger_id: inner.ledger_id,
        records,
        next_cursor: LedgerCursor {
            ledger_id: inner.ledger_id,
            after_seq: next_after_seq,
        },
        coverage: EventReadCoverage {
            retained_from,
            tip_seq,
            scanned_from_seq,
            scanned_through_seq,
            truncation,
        },
    })
}

fn validate_identity(
    expected: LedgerIdentity,
    actual: LedgerIdentity,
) -> Result<(), EventAuthorityError> {
    if expected == actual {
        Ok(())
    } else {
        Err(EventAuthorityError::IdentityMismatch { expected, actual })
    }
}

struct AuthorityLease {
    ledger_id: LedgerIdentity,
}

impl AuthorityLease {
    fn acquire(ledger_id: LedgerIdentity) -> Result<Self, EventAuthorityError> {
        let mut leases = authority_leases()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !leases.insert(ledger_id) {
            return Err(EventAuthorityError::DuplicateAuthorityBinding { ledger_id });
        }
        Ok(Self { ledger_id })
    }
}

impl Drop for AuthorityLease {
    fn drop(&mut self) {
        authority_leases()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.ledger_id);
    }
}

fn authority_leases() -> &'static Mutex<HashSet<LedgerIdentity>> {
    static LEASES: OnceLock<Mutex<HashSet<LedgerIdentity>>> = OnceLock::new();
    LEASES.get_or_init(|| Mutex::new(HashSet::new()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn failed_flush_returns_indeterminate_without_advancing_watermark() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
        authority.inner.store.fail_next_flush_for_test();

        let error = authority
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-07-10T00:00:00Z".to_string(),
                    "flush-test",
                    "execution",
                    "flush-test",
                    "execution.flush_test",
                    None,
                    json!({}),
                ),
                AppendMode::Plain,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            EventAuthorityError::DurabilityIndeterminate { .. }
        ));
        let watermark = authority.watermark_capability().snapshot().unwrap();
        assert_eq!(watermark.committed_seq, 0);
        assert_eq!(watermark.tip_seq, 1);
    }
}
