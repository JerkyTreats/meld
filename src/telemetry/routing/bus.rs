//! In-process event bus for telemetry events.

use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

use serde_json::Value;

use crate::telemetry::events::ProgressEnvelope;

const DEFAULT_PROGRESS_BUS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct ProgressBus {
    sender: SyncSender<ProgressEnvelope>,
}

impl ProgressBus {
    pub fn new_pair() -> (Self, Receiver<ProgressEnvelope>) {
        Self::new_pair_with_capacity(DEFAULT_PROGRESS_BUS_CAPACITY)
    }

    pub fn new_pair_with_capacity(capacity: usize) -> (Self, Receiver<ProgressEnvelope>) {
        let (sender, receiver) = sync_channel(capacity);
        (Self { sender }, receiver)
    }

    pub fn emit_envelope(
        &self,
        envelope: ProgressEnvelope,
    ) -> Result<(), TrySendError<ProgressEnvelope>> {
        self.sender.try_send(envelope)
    }

    pub fn emit(
        &self,
        session: impl Into<String>,
        event_type: impl Into<String>,
        data: Value,
    ) -> Result<(), TrySendError<ProgressEnvelope>> {
        let envelope = ProgressEnvelope::with_now(session, event_type, data);
        self.emit_envelope(envelope)
    }
}
