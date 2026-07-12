//! Event domain reexports and CLI tooling adapter.

pub use meld_events::*;

/// Product identity to event-authority binding and legacy cutover orchestration.
pub mod binding;
/// Event ledger observability CLI adapter.
pub mod tooling;
