//! Single-writer ingress engine for the event ledger.
//!
//! Owner: event ingress.
//! Inputs: producer envelopes with a durability class.
//! Outputs: atomically sequenced, group-committed records in the backing
//! [`crate::events::store::EventStore`], durable acks resolved after fsync,
//! a committed-sequence watermark, and drop diagnostics.
//! Does not own: this module does not interpret payloads, route telemetry,
//! or persist consumer cursors.
//!
//! One dedicated thread owns all producer-side appends. Requests queue on a
//! bounded channel; the thread drains whatever is pending, persists the
//! batch, then flushes once, so concurrent producers share fsyncs instead of
//! serializing on them. Durable requests receive their ack only after the
//! flush that made them durable. Consumer-side appends such as graph derived
//! events may still write to the store directly; the watermark therefore
//! tracks writer-committed sequences, not the store tail.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use tracing::warn;

use crate::error::StorageError;
use crate::events::store::{EventStore, StoreAppendOutcome};
use crate::events::EventEnvelope;

const WRITER_QUEUE_CAPACITY: usize = 1024;
const MAX_COMMIT_BATCH: usize = 256;

/// Highest sequence the writer has durably committed.
///
/// Consumers block on [`CommitWatermark::wait_past`] instead of polling the
/// store. Sequences appended outside the writer do not advance it.
pub struct CommitWatermark {
    committed: Mutex<u64>,
    changed: Condvar,
}

impl CommitWatermark {
    fn with_initial(committed: u64) -> Self {
        Self {
            committed: Mutex::new(committed),
            changed: Condvar::new(),
        }
    }

    /// Returns the highest writer-committed sequence.
    pub fn committed_seq(&self) -> u64 {
        *self.committed.lock().expect("watermark lock poisoned")
    }

    /// Blocks until the watermark passes `seq` or the timeout elapses, and
    /// returns the watermark at wake.
    pub fn wait_past(&self, seq: u64, timeout: Duration) -> u64 {
        let guard = self.committed.lock().expect("watermark lock poisoned");
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, timeout, |committed| *committed <= seq)
            .expect("watermark lock poisoned");
        *guard
    }

    fn advance(&self, seq: u64) {
        let mut guard = self.committed.lock().expect("watermark lock poisoned");
        if seq > *guard {
            *guard = seq;
            self.changed.notify_all();
        }
    }
}

enum WriteRequest {
    Append {
        envelope: Box<EventEnvelope>,
        idempotent: bool,
        ack: Option<SyncSender<Result<StoreAppendOutcome, StorageError>>>,
    },
    Barrier(SyncSender<()>),
    Shutdown,
}

/// Owner of the ledger writer thread and its producer-facing queue.
///
/// Dropping the writer drains every queued request, flushes, and joins the
/// thread, so short-lived processes never lose acked events.
pub struct EventWriter {
    sender: SyncSender<WriteRequest>,
    watermark: Arc<CommitWatermark>,
    dropped: Arc<AtomicU64>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl EventWriter {
    /// Spawns the writer thread over a shared store.
    pub fn spawn(store: Arc<EventStore>) -> Self {
        // TODO compat-shim: E5 removes direct writer spawning after authority
        // append parity, recovery, and route-level one-sequence tests pass.
        Self::spawn_with_watermark(store, 0)
    }

    /// Spawns the authority-owned writer at the recovered durable tip.
    pub(crate) fn spawn_recovered(store: Arc<EventStore>, committed: u64) -> Self {
        Self::spawn_with_watermark(store, committed)
    }

    fn spawn_with_watermark(store: Arc<EventStore>, committed: u64) -> Self {
        let (sender, receiver) = sync_channel(WRITER_QUEUE_CAPACITY);
        let watermark = Arc::new(CommitWatermark::with_initial(committed));
        let thread_watermark = Arc::clone(&watermark);
        let join = std::thread::Builder::new()
            .name("meld-event-writer".to_string())
            .spawn(move || run_writer(store, receiver, thread_watermark))
            .expect("spawn ledger writer thread");
        Self {
            sender,
            watermark,
            dropped: Arc::new(AtomicU64::new(0)),
            join: Mutex::new(Some(join)),
        }
    }

    /// Appends and blocks until the record is fsynced, returning its sequence.
    ///
    /// An error after a failed flush means the record may still become
    /// durable through a later successful commit: group commit cannot
    /// distinguish lost from delayed persistence. Retrying with a
    /// `record_id` through the idempotent path is safe; retrying a plain
    /// append may duplicate the event.
    pub fn append_durable(
        &self,
        envelope: EventEnvelope,
        idempotent: bool,
    ) -> Result<u64, StorageError> {
        self.append_durable_outcome(envelope, idempotent)
            .map(|outcome| outcome.seq)
    }

    /// Appends durably and reports whether idempotency inserted a new row.
    pub(crate) fn append_durable_outcome(
        &self,
        envelope: EventEnvelope,
        idempotent: bool,
    ) -> Result<StoreAppendOutcome, StorageError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.sender
            .send(WriteRequest::Append {
                envelope: Box::new(envelope),
                idempotent,
                ack: Some(ack_sender),
            })
            .map_err(|_| disconnected())?;
        ack_receiver.recv().map_err(|_| disconnected())?
    }

    /// Appends a batch, enqueueing everything before waiting so the whole
    /// batch shares the writer's group commits, and returns all sequences.
    pub fn append_durable_batch(
        &self,
        envelopes: Vec<EventEnvelope>,
        idempotent: bool,
    ) -> Result<Vec<u64>, StorageError> {
        self.append_durable_outcomes_batch(envelopes, idempotent)
            .map(|outcomes| outcomes.into_iter().map(|outcome| outcome.seq).collect())
    }

    /// Appends a durable batch and preserves each idempotency disposition.
    pub(crate) fn append_durable_outcomes_batch(
        &self,
        envelopes: Vec<EventEnvelope>,
        idempotent: bool,
    ) -> Result<Vec<StoreAppendOutcome>, StorageError> {
        let mut receivers = Vec::with_capacity(envelopes.len());
        for envelope in envelopes {
            let (ack_sender, ack_receiver) = sync_channel(1);
            self.sender
                .send(WriteRequest::Append {
                    envelope: Box::new(envelope),
                    idempotent,
                    ack: Some(ack_sender),
                })
                .map_err(|_| disconnected())?;
            receivers.push(ack_receiver);
        }
        receivers
            .into_iter()
            .map(|receiver| receiver.recv().map_err(|_| disconnected())?)
            .collect()
    }

    /// Enqueues without waiting; a full queue drops the event and counts it.
    pub fn append_best_effort(
        &self,
        envelope: EventEnvelope,
        idempotent: bool,
    ) -> Result<(), StorageError> {
        self.enqueue_best_effort(envelope, idempotent)
    }

    /// Enqueues one authority append without claiming durability or sequence.
    pub(crate) fn enqueue_best_effort(
        &self,
        envelope: EventEnvelope,
        idempotent: bool,
    ) -> Result<(), StorageError> {
        let request = WriteRequest::Append {
            envelope: Box::new(envelope),
            idempotent,
            ack: None,
        };
        match self.sender.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                let dropped = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
                if dropped.is_power_of_two() {
                    warn!(
                        dropped,
                        "ledger writer queue full; dropping best-effort events"
                    );
                }
                Err(StorageError::Backpressure(format!(
                    "ledger writer queue full; {dropped} best-effort events dropped"
                )))
            }
            Err(TrySendError::Disconnected(_)) => Err(disconnected()),
        }
    }

    /// Blocks until everything enqueued before this call has been processed
    /// and a flush attempted.
    ///
    /// The queue is ordered, so the barrier ack proves every earlier append,
    /// including best-effort ones, has reached the store; callers needing a
    /// durability guarantee per event use the durable append path instead.
    pub fn barrier(&self) -> Result<(), StorageError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.sender
            .send(WriteRequest::Barrier(ack_sender))
            .map_err(|_| disconnected())?;
        ack_receiver.recv().map_err(|_| disconnected())
    }

    /// Returns the shared committed-sequence watermark.
    pub fn watermark(&self) -> Arc<CommitWatermark> {
        // TODO compat-shim: E5 removes this raw handle after all production
        // callers use EventWatermarkCapability and recovery parity is green.
        Arc::clone(&self.watermark)
    }

    /// Returns how many best-effort events have been dropped by backpressure.
    pub fn dropped_events(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Returns the shared drop counter for observability backings.
    pub fn dropped_handle(&self) -> Arc<AtomicU64> {
        // TODO compat-shim: E5 removes this raw handle after authority-backed
        // observability reports drops with report/CLI parity tests.
        Arc::clone(&self.dropped)
    }
}

impl Drop for EventWriter {
    fn drop(&mut self) {
        let _ = self.sender.send(WriteRequest::Shutdown);
        if let Some(join) = self
            .join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = join.join();
        }
    }
}

fn run_writer(
    store: Arc<EventStore>,
    receiver: Receiver<WriteRequest>,
    watermark: Arc<CommitWatermark>,
) {
    let mut shutting_down = false;
    while !shutting_down {
        let first = match receiver.recv() {
            Ok(request) => request,
            Err(_) => break,
        };

        // Group commit by natural batching: take whatever is queued right
        // now, persist it all, and pay one flush. Under contention batches
        // grow and fsyncs amortize; when idle a single event flushes
        // immediately, so latency never waits on a timer.
        let mut batch = Vec::with_capacity(16);
        match first {
            WriteRequest::Shutdown => shutting_down = true,
            request => batch.push(request),
        }
        while batch.len() < MAX_COMMIT_BATCH && !shutting_down {
            match receiver.try_recv() {
                Ok(WriteRequest::Shutdown) => shutting_down = true,
                Ok(request) => batch.push(request),
                Err(_) => break,
            }
        }
        commit_batch(&store, &watermark, batch);
    }

    // Drain-on-shutdown: everything already queued still commits before the
    // thread exits, so drop-time flushes keep one-shot CLI processes safe.
    let mut remaining = Vec::new();
    while let Ok(request) = receiver.try_recv() {
        if !matches!(request, WriteRequest::Shutdown) {
            remaining.push(request);
        }
    }
    commit_batch(&store, &watermark, remaining);
}

fn commit_batch(store: &EventStore, watermark: &CommitWatermark, batch: Vec<WriteRequest>) {
    if batch.is_empty() {
        return;
    }

    let mut acks = Vec::new();
    let mut barriers = Vec::new();
    let mut max_seq = 0u64;
    let mut appends = 0usize;
    for request in batch {
        let (envelope, idempotent, ack) = match request {
            WriteRequest::Append {
                envelope,
                idempotent,
                ack,
            } => (envelope, idempotent, ack),
            WriteRequest::Barrier(barrier) => {
                barriers.push(barrier);
                continue;
            }
            WriteRequest::Shutdown => continue,
        };
        appends += 1;
        let result = store.append_envelope_outcome(*envelope, idempotent);
        if let Ok(outcome) = result.as_ref() {
            max_seq = max_seq.max(outcome.seq);
        }
        match ack {
            Some(ack) => acks.push((ack, result)),
            None => {
                if let Err(error) = result {
                    warn!(error = %error, "best-effort ledger append failed");
                }
            }
        }
    }

    // A barrier-only batch has nothing new to flush: earlier batches
    // already flushed their appends before acking.
    let flush_result = if appends > 0 { store.flush() } else { Ok(()) };

    // The watermark advances before any ack releases a producer, so an
    // acked caller can always observe a watermark at or past its sequence.
    if flush_result.is_ok() && max_seq > 0 {
        watermark.advance(max_seq);
    }

    // A durable ack means durable bytes: successful appends are downgraded
    // to the flush error when the fsync itself failed.
    for (ack, result) in acks {
        let final_result = match (&flush_result, result) {
            (Err(flush_error), Ok(_)) => Err(StorageError::DurabilityIndeterminate(
                flush_error.to_string(),
            )),
            (_, result) => result,
        };
        let _ = ack.send(final_result);
    }

    for barrier in barriers {
        let _ = barrier.send(());
    }
}

fn disconnected() -> StorageError {
    StorageError::Unavailable("ledger writer disconnected".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_writer() -> (tempfile::TempDir, Arc<EventStore>, EventWriter) {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::shared(db).unwrap();
        let writer = EventWriter::spawn(Arc::clone(&store));
        (dir, store, writer)
    }

    fn envelope(i: usize) -> EventEnvelope {
        EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "session-w",
            "execution",
            "session-w",
            "execution.task.progress",
            None,
            json!({ "i": i }),
        )
    }

    #[test]
    fn durable_append_acks_with_sequence_and_advances_watermark() {
        let (_dir, _store, writer) = temp_writer();
        let seq = writer.append_durable(envelope(0), false).unwrap();
        assert_eq!(seq, 1);
        assert_eq!(writer.watermark().committed_seq(), 1);
    }

    #[test]
    fn concurrent_durable_appends_receive_distinct_sequences() {
        let (_dir, store, writer) = temp_writer();
        let writer = Arc::new(writer);
        let handles: Vec<_> = (0..8)
            .map(|t| {
                let writer = Arc::clone(&writer);
                std::thread::spawn(move || {
                    (0..50)
                        .map(|i| {
                            writer
                                .append_durable(envelope(t * 1000 + i), false)
                                .unwrap()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut seqs: Vec<u64> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();
        seqs.sort_unstable();
        seqs.dedup();
        assert_eq!(seqs.len(), 400);
        assert_eq!(store.read_all_events_after(0).unwrap().len(), 400);
    }

    #[test]
    fn drop_drains_queued_best_effort_events() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::shared(db).unwrap();
        {
            let writer = EventWriter::spawn(Arc::clone(&store));
            for i in 0..64 {
                writer.append_best_effort(envelope(i), false).unwrap();
            }
        }
        assert_eq!(store.read_all_events_after(0).unwrap().len(), 64);
    }

    #[test]
    fn idempotent_durable_appends_settle_to_one_record() {
        let (_dir, store, writer) = temp_writer();
        let tagged = envelope(0).with_record_id("record-w");
        let first = writer.append_durable(tagged.clone(), true).unwrap();
        let second = writer.append_durable(tagged, true).unwrap();
        assert_eq!(first, second);
        assert_eq!(store.read_all_events_after(0).unwrap().len(), 1);
    }

    #[test]
    fn full_queue_drops_best_effort_and_counts() {
        // Built without a writer thread so the queue stays full
        // deterministically; capacity one means the second enqueue must drop.
        let (sender, receiver) = sync_channel(1);
        let writer = EventWriter {
            sender,
            watermark: Arc::new(CommitWatermark::with_initial(0)),
            dropped: Arc::new(AtomicU64::new(0)),
            join: Mutex::new(None),
        };

        writer.append_best_effort(envelope(0), false).unwrap();
        let err = writer.append_best_effort(envelope(1), false).unwrap_err();
        assert!(matches!(err, StorageError::Backpressure(_)));
        assert_eq!(writer.dropped_events(), 1);
        assert!(writer.append_best_effort(envelope(2), false).is_err());
        assert_eq!(writer.dropped_events(), 2);

        // Drained so the drop-time shutdown send cannot block on the full
        // queue.
        while receiver.try_recv().is_ok() {}
    }

    #[test]
    fn barrier_observes_prior_best_effort_appends() {
        let (_dir, store, writer) = temp_writer();
        for i in 0..16 {
            writer.append_best_effort(envelope(i), false).unwrap();
        }
        writer.barrier().unwrap();
        assert_eq!(store.read_all_events_after(0).unwrap().len(), 16);
    }

    #[test]
    fn wait_past_wakes_on_commit() {
        let (_dir, _store, writer) = temp_writer();
        let watermark = writer.watermark();
        let waiter = std::thread::spawn(move || watermark.wait_past(0, Duration::from_secs(5)));
        writer.append_durable(envelope(0), false).unwrap();
        assert!(waiter.join().unwrap() >= 1);
    }
}
