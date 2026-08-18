//! Runtime assembly diagnostics and supervisor-facing contracts.

/// Assignment-local PDS startup activation and durable generation heads.
pub mod activation;
/// Product runtime assembly entrypoint.
pub mod assembly;
/// Supervisor-facing worker report contracts.
pub mod contracts;
/// Passive PDS source subscriptions and fenced delivery lineage.
pub mod delivery;
/// Runtime assembly and port error surfaces.
pub mod error;
/// Portable PDS generation lifecycle and durable external operations.
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
