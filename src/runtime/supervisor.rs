//! Root runtime supervisor domain.

/// Durable supervisor lifecycle store.
pub mod store;

pub use store::{SupervisorStore, SupervisorStoreError};
