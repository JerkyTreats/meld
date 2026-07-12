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
//! - [`events`] defines canonical envelopes, records, authority capabilities,
//!   observability, migration, and transport-neutral contracts.
//! - [`error`] defines persistence and authority errors.
//!
//! Product composition opens one [`EventAuthority`]. Producers use its
//! [`EventAppendCapability`], while consumers derive identity-bound replay,
//! subscription, watermark, registry, and observability capabilities.
//!
//! # Example
//!
//! ```rust
//! use meld_events::{AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope};
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
//! let db = sled::Config::new().temporary(true).open().unwrap();
//! let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
//! let receipt = authority
//!     .append_capability()
//!     .append_durable(envelope, AppendMode::Plain)
//!     .unwrap();
//!
//! assert_eq!(receipt.seq, 1);
//! assert_eq!(receipt.ledger_id, authority.ledger_identity());
//! ```

#![deny(missing_docs)]

pub mod error;
pub mod events;

pub use events::*;
