//! Runtime assembly diagnostics and supervisor-facing contracts.

/// Product activation loading and owner-scoped input contracts.
pub mod activation;
/// Product runtime assembly entrypoint.
pub mod assembly;
/// Supervisor-facing worker report contracts.
pub mod contracts;
/// Runtime assembly and port error surfaces.
pub mod error;
/// Passive runtime status cache projection and presentation.
pub mod passive_status;
/// Thin direct handoff ports built by product assembly.
pub mod ports;
/// Runtime CLI presentation helpers.
pub mod presentation;
/// Promoted runtime-health facts emitted by the threshold watcher.
pub mod self_observation;
/// Bounded filesystem host for passive runtime status.
pub mod status_cache;
/// Product storage assembly for durable runtime stores.
pub mod storage;
/// Root supervisor lifecycle storage.
pub mod supervisor;
/// Bounded task-network dispatch into concrete execution routes.
pub mod task_dispatch;
/// Runtime CLI adapter.
pub mod tooling;
