//! Runtime assembly diagnostics and supervisor-facing contracts.

/// Product runtime assembly entrypoint.
pub mod assembly;
/// Supervisor-facing worker report contracts.
pub mod contracts;
/// Owner preparation for an exact native Agent epoch specification.
pub mod epoch;
/// Runtime assembly and port error surfaces.
pub mod error;
/// Canonical assignment generation and structural lifecycle authority.
pub mod lifecycle;
/// Thin direct handoff ports built by product assembly.
pub mod ports;
/// Runtime CLI presentation helpers.
pub mod presentation;
pub mod registration;
/// Promoted runtime-health facts emitted by the threshold watcher.
pub mod self_observation;
/// Product storage assembly for durable runtime stores.
pub mod storage;
/// Root supervisor lifecycle storage.
pub mod supervisor;
/// Durable theory installation receipts and exact resolution.
pub mod theory;
/// Runtime CLI adapter.
pub mod tooling;
