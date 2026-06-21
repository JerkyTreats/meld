//! Runtime assembly diagnostics and supervisor-facing contracts.

/// Product runtime assembly entrypoint.
pub mod assembly;
/// Supervisor-facing worker report contracts.
pub mod contracts;
/// Runtime assembly and port error surfaces.
pub mod error;
/// Thin direct handoff ports built by product assembly.
pub mod ports;
/// Product storage assembly for durable runtime stores.
pub mod storage;
/// Root supervisor lifecycle storage.
pub mod supervisor;
