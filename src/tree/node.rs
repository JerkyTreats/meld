//! Filesystem node types and NodeID computation

use crate::types::NodeID;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// File node representation
#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub content_hash: [u8; 32],
    pub size: u64,
    pub metadata: BTreeMap<String, String>,
}

/// Directory node representation
#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub path: PathBuf,
    pub children: Vec<(String, NodeID)>, // (name, node_id) sorted by name
    pub metadata: BTreeMap<String, String>,
}

/// Merkle node type
#[derive(Debug, Clone)]
pub enum MerkleNode {
    File(FileNode),
    Directory(DirectoryNode),
}
