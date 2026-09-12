//! Runtime assembly diagnostics and supervisor-facing contracts.

/// Product runtime assembly entrypoint.
pub mod assembly;
/// Supervisor-facing worker report contracts.
pub mod contracts;
/// Public operational control over the existing supervisor.
pub mod control;
/// Owner preparation for an exact native Agent epoch specification.
pub mod epoch;
/// Runtime assembly and port error surfaces.
pub mod error;
/// Canonical assignment generation and structural lifecycle authority.
pub mod lifecycle;
/// Managed command process launch and observation.
pub mod managed;
/// Serialized host connections for external package owners.
pub mod owners;
/// Thin direct handoff ports built by product assembly.
pub mod ports;
mod positions;
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
pub(crate) use positions::{
    bind_product_positions, prepared_product_positions, BoundProductPosition,
};
/// Runtime CLI adapter.
pub mod tooling;
