//! NodeRecord Store
//!
//! Provides fast lookup storage for node metadata and relationships.
//! Acts as an index into the filesystem Merkle tree.

pub mod persistence;

use crate::error::StorageError;
use crate::types::{Hash, NodeID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Node type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    File { size: u64, content_hash: [u8; 32] },
    Directory,
}

/// NodeRecord: Metadata and relationships for a filesystem node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub node_id: NodeID,
    pub path: PathBuf,
    pub node_type: NodeType,
    pub children: Vec<NodeID>,
    pub parent: Option<NodeID>,
    pub frame_set_root: Option<Hash>,
    pub metadata: HashMap<String, String>,
}

/// NodeRecord Store interface
pub trait NodeRecordStore {
    fn get(&self, node_id: &NodeID) -> Result<Option<NodeRecord>, StorageError>;
    fn put(&self, record: &NodeRecord) -> Result<(), StorageError>;
}
