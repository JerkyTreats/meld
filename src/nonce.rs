//! Reusable, deterministic owner-issued nonce publication.
//!
//! The complete request and the Event ledger are the reconstruction sources.
//! This owner has no Startup, Goal, lifecycle, or health semantics and no separate store.

pub mod capability;
mod contracts;
mod publication;
#[cfg(test)]
mod tests;

pub use contracts::*;
pub use publication::*;
