//! A read-only handle to the live network's committed reduction.

use std::sync::{Arc, Mutex};

use super::{SledTaskNetworkStore, TaskNetworkStoreError};
use crate::task_network::state::NetworkState;

/// Inspection shares the existing store lock and cannot submit commands or open a writer.
#[derive(Clone)]
pub struct TaskNetworkReader {
    store: Arc<Mutex<SledTaskNetworkStore>>,
}

impl TaskNetworkReader {
    /// Bind inspection to an already opened native network.
    pub fn new(store: Arc<Mutex<SledTaskNetworkStore>>) -> Self {
        Self { store }
    }

    /// Copy one complete committed reduction while holding the owner's store lock.
    pub fn snapshot(&self) -> Result<NetworkState, TaskNetworkStoreError> {
        self.store
            .lock()
            .map(|store| store.state().clone())
            .map_err(|_| TaskNetworkStoreError::Storage("Task Network lock is poisoned".into()))
    }
}
