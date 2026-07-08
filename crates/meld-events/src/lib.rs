//! Canonical event ledger contracts and runtime helpers for Meld.
//!
//! This crate owns event envelopes, sequenced event records, domain object
//! references carried by events, the single-writer ingress engine, and the
//! sled-backed append-only event store.
//!
//! This crate does not own task execution, workflow orchestration, world model
//! materialization, or telemetry sink routing. Those domains publish events into
//! this crate and consume sequenced records through explicit contracts.
//!
//! # Module Map
//!
//! - [`events`] defines the canonical envelope, record, runtime, store, and
//!   compatibility surface.
//! - [`error`] defines storage and API errors raised by event persistence and
//!   runtime emission.
//!
//! Start with [`EventEnvelope`] when publishing a domain event,
//! [`events::store::EventStore`] when persisting or querying the event ledger,
//! and [`EventRuntime`] when a caller needs durable or best-effort emission.
//!
//! # Example
//!
//! ```rust
//! use meld_events::{EventEnvelope, EventRecord};
//! use serde_json::json;
//!
//! let envelope = EventEnvelope::new_domain(
//!     "2026-04-26T16:00:00Z".to_string(),
//!     "session-a",
//!     "execution",
//!     "workflow-a",
//!     "execution.started",
//!     None,
//!     json!({ "started": true }),
//! );
//! let record = EventRecord::from_envelope(envelope, 1);
//!
//! assert_eq!(record.seq, 1);
//! assert_eq!(record.domain_id, "execution");
//! ```

#![deny(missing_docs)]

pub mod error;
pub mod events;

pub use events::*;
