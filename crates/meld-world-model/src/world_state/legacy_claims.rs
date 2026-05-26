//! Legacy claim namespace.
//!
//! This module groups the claim-oriented contracts, reducer, query, and store
//! while the top-level `world_state` module also exposes the newer graph domain.

pub use super::contracts;
pub use super::events;
pub use super::projection;
pub use super::query;
pub use super::reducer;
pub use super::store;
