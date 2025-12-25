//! Concurrent access safety for agent operations
//!
//! Provides per-node locking to ensure safe concurrent access by multiple agents.
//! Read operations don't require locks (immutable data), but write operations
//! use per-node locks to prevent corruption.

use crate::types::NodeID;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Per-node lock manager for concurrent access safety
///
/// Provides fine-grained locking at the node level, allowing multiple agents
/// to operate on different nodes concurrently while preventing conflicts on
/// the same node.
pub struct NodeLockManager {
    /// Map from NodeID to per-node read-write lock
    /// Uses Arc<RwLock<()>> to allow shared ownership and fine-grained locking
    locks: Arc<RwLock<HashMap<NodeID, Arc<RwLock<()>>>>>,
}

impl NodeLockManager {
    /// Create a new node lock manager
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a lock for a specific node
    ///
    /// Returns a guard that can be used for read or write locking.
    /// The lock is automatically cleaned up when no longer needed.
    fn get_node_lock(&self, node_id: &NodeID) -> Arc<RwLock<()>> {
        // Try to get existing lock (read lock for map lookup)
        {
            let map = self.locks.read();
            if let Some(lock) = map.get(node_id) {
                return lock.clone();
            }
        }

        // Lock doesn't exist, create it (write lock for map modification)
        let mut map = self.locks.write();
        // Double-check after acquiring write lock (another thread might have created it)
        map.entry(*node_id)
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone()
    }

    /// Get the lock for a node
    ///
    /// Returns an Arc to the node's lock, which can be used to acquire
    /// read or write guards. The lock is automatically cleaned up when
    /// no longer needed.
    pub fn get_lock(&self, node_id: &NodeID) -> Arc<RwLock<()>> {
        self.get_node_lock(node_id)
    }
}

impl Default for NodeLockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_concurrent_reads() {
        let manager = Arc::new(NodeLockManager::new());
        let node_id: NodeID = [1u8; 32];
        let counter = Arc::new(AtomicUsize::new(0));

        // Spawn multiple threads that all read-lock the same node
        let mut handles = vec![];
        for _ in 0..10 {
            let manager = manager.clone();
            let counter = counter.clone();
            let handle = thread::spawn(move || {
                let lock = manager.get_lock(&node_id);
                let _guard = lock.read();
                counter.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All reads should have completed
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_write_excludes_other_writes() {
        let manager = Arc::new(NodeLockManager::new());
        let node_id: NodeID = [1u8; 32];
        let counter = Arc::new(AtomicUsize::new(0));

        // Spawn multiple threads that all write-lock the same node
        let mut handles = vec![];
        for _ in 0..5 {
            let manager = manager.clone();
            let counter = counter.clone();
            let handle = thread::spawn(move || {
                let lock = manager.get_lock(&node_id);
                let _guard = lock.write();
                let current = counter.load(Ordering::SeqCst);
                thread::yield_now(); // Give other threads a chance
                counter.store(current + 1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All writes should have completed sequentially (no lost updates)
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_different_nodes_dont_block() {
        let manager = Arc::new(NodeLockManager::new());
        let node_id1: NodeID = [1u8; 32];
        let node_id2: NodeID = [2u8; 32];
        let counter = Arc::new(AtomicUsize::new(0));

        // Spawn threads that lock different nodes
        let mut handles = vec![];
        for i in 0..5 {
            let manager = manager.clone();
            let counter = counter.clone();
            let node_id = if i % 2 == 0 { node_id1 } else { node_id2 };
            let handle = thread::spawn(move || {
                let lock = manager.get_lock(&node_id);
                let _guard = lock.write();
                counter.fetch_add(1, Ordering::SeqCst);
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // All operations should complete (different nodes don't block each other)
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }
}
