//! Task network command stores.
//!
//! Owner: task network.
//! Inputs: command requests and durable journal records.
//! Outputs: command responses, journal records, and reduced network state.
//! Does not own: this module does not execute tasks or publish external
//! events.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::store::InMemoryTaskNetworkStore;
//!
//! let store = InMemoryTaskNetworkStore::new("network-a");
//! assert_eq!(store.state().revision, 0);
//! ```

mod codec;
mod error;
mod factory;
mod memory;
mod records;
mod sled;

pub use crate::task_network::journal::JournalRecord;
pub use error::TaskNetworkStoreError;
pub use factory::{network_storage_key, TaskNetworkStoreFactory};
pub use memory::InMemoryTaskNetworkStore;
pub use sled::SledTaskNetworkStore;
