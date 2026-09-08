//! Context-owned durable current frame selections.
//!
//! Graph consumes publications of this source store. It never selects frame heads.

use crate::error::StorageError;
use crate::types::{FrameID, NodeID};
use bincode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const HEAD_INDEX_VERSION_V1: u32 = 1;
const HEAD_INDEX_VERSION_V2: u32 = 2;
const HEAD_INDEX_VERSION_V3: u32 = 3;

/// Head entry with optional tombstone marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HeadEntry {
    pub frame_id: FrameID,
    pub tombstoned_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadSelection {
    pub node_id: NodeID,
    pub frame_type: String,
    pub frame_id: FrameID,
    pub tombstoned_at: Option<u64>,
}

/// Head index: (NodeID, frame_type) -> HeadEntry
pub struct HeadIndex {
    source_id: String,
    revision_id: String,
    pub(crate) heads: HashMap<(NodeID, String), HeadEntry>,
}

impl Default for HeadIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadIndex {
    pub fn new() -> Self {
        HeadIndex {
            source_id: uuid::Uuid::new_v4().to_string(),
            revision_id: uuid::Uuid::new_v4().to_string(),
            heads: HashMap::new(),
        }
    }

    /// Get active head for node and frame type (skips tombstoned entries).
    pub fn get_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, StorageError> {
        Ok(self
            .heads
            .get(&(*node_id, frame_type.to_string()))
            .filter(|e| e.tombstoned_at.is_none())
            .map(|e| e.frame_id))
    }

    /// Get active head (alias for get_head; skips tombstoned).
    pub fn get_active_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, StorageError> {
        self.get_head(node_id, frame_type)
    }

    pub fn update_head(
        &mut self,
        node_id: &NodeID,
        frame_type: &str,
        frame_id: &FrameID,
    ) -> Result<(), StorageError> {
        self.revision_id = uuid::Uuid::new_v4().to_string();
        self.heads.insert(
            (*node_id, frame_type.to_string()),
            HeadEntry {
                frame_id: *frame_id,
                tombstoned_at: None,
            },
        );
        Ok(())
    }

    /// Tombstone all head entries for a node (all frame types).
    pub fn tombstone_heads_for_node(&mut self, node_id: &NodeID) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.revision_id = uuid::Uuid::new_v4().to_string();
        for ((nid, _), entry) in self.heads.iter_mut() {
            if *nid == *node_id {
                entry.tombstoned_at = Some(now);
            }
        }
    }

    /// Tombstone a single head entry for a node and frame type.
    pub fn tombstone_head(&mut self, node_id: &NodeID, frame_type: &str) -> Option<FrameID> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.revision_id = uuid::Uuid::new_v4().to_string();
        let key = (*node_id, frame_type.to_string());
        self.heads.get_mut(&key).map(|entry| {
            entry.tombstoned_at = Some(now);
            entry.frame_id
        })
    }

    /// Restore all head entries for a node (remove tombstone marker).
    pub fn restore_heads_for_node(&mut self, node_id: &NodeID) {
        self.revision_id = uuid::Uuid::new_v4().to_string();
        for ((nid, _), entry) in self.heads.iter_mut() {
            if *nid == *node_id {
                entry.tombstoned_at = None;
            }
        }
    }

    /// Purge tombstoned head entries older than cutoff.
    pub fn purge_tombstoned(&mut self, cutoff: u64) {
        self.revision_id = uuid::Uuid::new_v4().to_string();
        self.heads
            .retain(|_, e| e.tombstoned_at.is_none_or(|ts| ts > cutoff));
    }

    /// Get all frame IDs for a given node (including tombstoned; used e.g. for compact).
    pub fn get_all_heads_for_node(&self, node_id: &NodeID) -> Vec<FrameID> {
        self.heads
            .iter()
            .filter_map(|((nid, _), e)| {
                if *nid == *node_id {
                    Some(e.frame_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all unique node IDs that have active (non-tombstoned) heads.
    pub fn get_all_node_ids(&self) -> Vec<NodeID> {
        let mut node_ids = std::collections::HashSet::new();
        for ((node_id, _), e) in &self.heads {
            if e.tombstoned_at.is_none() {
                node_ids.insert(*node_id);
            }
        }
        node_ids.into_iter().collect()
    }

    /// Count distinct node IDs that have an active head for the given frame type.
    pub fn count_nodes_for_frame_type(&self, frame_type: &str) -> usize {
        let mut node_ids = std::collections::HashSet::new();
        for ((node_id, ft), e) in &self.heads {
            if ft.as_str() == frame_type && e.tombstoned_at.is_none() {
                node_ids.insert(*node_id);
            }
        }
        node_ids.len()
    }

    pub fn active_entries(&self) -> Vec<HeadSelection> {
        self.heads
            .iter()
            .filter_map(|((node_id, frame_type), entry)| {
                if entry.tombstoned_at.is_some() {
                    return None;
                }
                Some(HeadSelection {
                    node_id: *node_id,
                    frame_type: frame_type.clone(),
                    frame_id: entry.frame_id,
                    tombstoned_at: entry.tombstoned_at,
                })
            })
            .collect()
    }

    pub fn entries_for_node(&self, node_id: &NodeID) -> Vec<HeadSelection> {
        self.heads
            .iter()
            .filter_map(|((nid, frame_type), entry)| {
                if nid != node_id {
                    return None;
                }
                Some(HeadSelection {
                    node_id: *nid,
                    frame_type: frame_type.clone(),
                    frame_id: entry.frame_id,
                    tombstoned_at: entry.tombstoned_at,
                })
            })
            .collect()
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }

    pub fn entries(&self) -> Vec<HeadSelection> {
        let mut entries = self
            .heads
            .iter()
            .map(|((node_id, frame_type), entry)| HeadSelection {
                node_id: *node_id,
                frame_type: frame_type.clone(),
                frame_id: entry.frame_id,
                tombstoned_at: entry.tombstoned_at,
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            (&left.node_id, &left.frame_type).cmp(&(&right.node_id, &right.frame_type))
        });
        entries
    }

    pub fn active_heads_for_node(&self, node_id: &NodeID) -> Vec<FrameID> {
        self.entries_for_node(node_id)
            .into_iter()
            .filter(|entry| entry.tombstoned_at.is_none())
            .map(|entry| entry.frame_id)
            .collect()
    }

    /// Get the persistence path for a workspace root
    ///
    /// Uses XDG data directory path for the workspace.
    /// If XDG resolution is unavailable, falls back to a temp data root outside the workspace.
    pub fn persistence_path(workspace_root: &Path) -> PathBuf {
        if let Ok(data_dir) = crate::config::xdg::workspace_data_dir(workspace_root) {
            data_dir.join("head_index.bin")
        } else {
            fallback_workspace_data_dir(workspace_root).join("head_index.bin")
        }
    }

    /// Load head index from disk
    ///
    /// Missing files start empty; corrupt data is an error.
    pub fn load_from_disk<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Ok(HeadIndex::new());
        }

        // Read file
        let bytes = fs::read(path).map_err(|e| {
            StorageError::IoError(std::io::Error::other(format!(
                "Failed to read head index from {:?}: {}",
                path, e
            )))
        })?;

        // Try legacy V1 format (single bincode blob) first.
        if let Ok(persistence) = bincode::deserialize::<HeadIndexPersistenceV1>(&bytes) {
            if persistence.version == HEAD_INDEX_VERSION_V1 {
                let mut heads = HashMap::new();
                for entry in persistence.entries {
                    if entry.frame_id.len() != 32 || entry.node_id.len() != 32 {
                        return Err(StorageError::IoError(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid frame_id or node_id length in head index".to_string(),
                        )));
                    }
                    let mut node_id = [0u8; 32];
                    node_id.copy_from_slice(&entry.node_id);
                    let mut frame_id = [0u8; 32];
                    frame_id.copy_from_slice(&entry.frame_id);
                    heads.insert(
                        (node_id, entry.frame_type),
                        HeadEntry {
                            frame_id,
                            tombstoned_at: None,
                        },
                    );
                }
                return Ok(HeadIndex {
                    heads,
                    source_id: format!(
                        "migrated-source-{}",
                        blake3::hash(path.to_string_lossy().as_bytes()).to_hex()
                    ),
                    revision_id: format!("migrated-{}", blake3::hash(&bytes).to_hex()),
                });
            }
        }

        // V2 format: 4-byte version then bincode(entries).
        if bytes.len() < 4 {
            return Err(StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Head index file too short".to_string(),
            )));
        }
        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if version == HEAD_INDEX_VERSION_V3 {
            let persisted: HeadIndexPersistenceV3 = bincode::deserialize(&bytes[4..])
                .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
            if persisted.revision_id.is_empty() || persisted.source_id.is_empty() {
                return Err(StorageError::InvalidPath(
                    "Context head source or revision is empty".into(),
                ));
            }
            return Ok(Self {
                heads: persisted.heads,
                source_id: persisted.source_id,
                revision_id: persisted.revision_id,
            });
        }
        if version != HEAD_INDEX_VERSION_V2 {
            return Err(StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported head index version: {}", version),
            )));
        }
        let entries: Vec<HeadIndexEntry> = bincode::deserialize(&bytes[4..]).map_err(|e| {
            StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to deserialize head index entries: {}", e),
            ))
        })?;

        let mut heads = HashMap::new();
        for entry in entries {
            if entry.frame_id.len() != 32 || entry.node_id.len() != 32 {
                return Err(StorageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid frame_id or node_id length in head index".to_string(),
                )));
            }
            let mut node_id = [0u8; 32];
            node_id.copy_from_slice(&entry.node_id);
            let mut frame_id = [0u8; 32];
            frame_id.copy_from_slice(&entry.frame_id);
            heads.insert(
                (node_id, entry.frame_type),
                HeadEntry {
                    frame_id,
                    tombstoned_at: entry.tombstoned_at,
                },
            );
        }

        Ok(HeadIndex {
            heads,
            source_id: format!(
                "migrated-source-{}",
                blake3::hash(path.to_string_lossy().as_bytes()).to_hex()
            ),
            revision_id: format!("migrated-{}", blake3::hash(&bytes).to_hex()),
        })
    }

    /// Save head index to disk atomically
    ///
    /// Uses temporary file + rename for atomic writes.
    pub fn save_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), StorageError> {
        let path = path.as_ref();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                StorageError::IoError(std::io::Error::other(format!(
                    "Failed to create parent directory {:?}: {}",
                    parent, e
                )))
            })?;
        }

        let payload = bincode::serialize(&HeadIndexPersistenceV3 {
            source_id: self.source_id.clone(),
            revision_id: self.revision_id.clone(),
            heads: self.heads.clone(),
        })
        .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let mut serialized = Vec::with_capacity(4 + payload.len());
        serialized.extend_from_slice(&HEAD_INDEX_VERSION_V3.to_le_bytes());
        serialized.extend_from_slice(&payload);

        // Write to temporary file (atomic write)
        let temp_path = path.with_extension("bin.tmp");
        use std::io::Write;
        (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&serialized)?;
            file.sync_all()
        })()
        .map_err(|e| {
            StorageError::IoError(std::io::Error::other(format!(
                "Failed to write head index to {:?}: {}",
                temp_path, e
            )))
        })?;

        // Atomically rename temp file to final location
        fs::rename(&temp_path, path).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_path);
            StorageError::IoError(std::io::Error::other(format!(
                "Failed to rename temp file to {:?}: {}",
                path, e
            )))
        })?;

        if let Some(parent) = path.parent() {
            fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(StorageError::IoError)?;
        }
        Ok(())
    }
}

fn fallback_workspace_data_dir(workspace_root: &Path) -> PathBuf {
    let canonical = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let mut data_dir = std::env::temp_dir().join("meld");

    for component in canonical.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(_) => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {}
            std::path::Component::Normal(name) => {
                data_dir = data_dir.join(name);
            }
        }
    }

    data_dir
}

#[derive(Serialize, Deserialize)]
struct HeadIndexPersistenceV3 {
    source_id: String,
    revision_id: String,
    heads: HashMap<(NodeID, String), HeadEntry>,
}

/// Persistence format for head index (version 1: legacy single-blob).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeadIndexPersistenceV1 {
    version: u32,
    entries: Vec<HeadIndexEntryV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeadIndexEntryV1 {
    node_id: Vec<u8>,
    frame_type: String,
    frame_id: Vec<u8>,
}

/// Entry in the head index persistence format (version 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeadIndexEntry {
    node_id: Vec<u8>,
    frame_type: String,
    frame_id: Vec<u8>,
    tombstoned_at: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_head_index() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("head_index.bin");

        // Create a head index with some entries
        let mut index = HeadIndex::new();
        let node_id: NodeID = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];
        index.update_head(&node_id, "test", &frame_id).unwrap();

        // Save to disk
        index.save_to_disk(&path).unwrap();
        assert!(path.exists());

        // Load from disk
        let loaded = HeadIndex::load_from_disk(&path).unwrap();
        assert_eq!(loaded.heads.len(), 1);
        assert_eq!(loaded.get_head(&node_id, "test").unwrap(), Some(frame_id));
    }

    #[test]
    fn test_load_nonexistent_head_index() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nonexistent.bin");

        // Load from non-existent file should return empty index
        let loaded = HeadIndex::load_from_disk(&path).unwrap();
        assert_eq!(loaded.heads.len(), 0);
    }

    #[test]
    fn test_persistence_path() {
        let workspace_root = std::path::Path::new("/workspace");
        let path = HeadIndex::persistence_path(workspace_root);
        assert!(path.to_string_lossy().ends_with("head_index.bin"));
        assert!(!path.to_string_lossy().contains("/workspace/.meld/"));
    }

    #[test]
    fn test_save_and_load_multiple_entries() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("head_index.bin");

        // Create a head index with multiple entries
        let mut index = HeadIndex::new();
        let node_id1: NodeID = [1u8; 32];
        let node_id2: NodeID = [2u8; 32];
        let frame_id1: FrameID = [10u8; 32];
        let frame_id2: FrameID = [20u8; 32];
        let frame_id3: FrameID = [30u8; 32];

        index.update_head(&node_id1, "type1", &frame_id1).unwrap();
        index.update_head(&node_id1, "type2", &frame_id2).unwrap();
        index.update_head(&node_id2, "type1", &frame_id3).unwrap();

        // Save to disk
        index.save_to_disk(&path).unwrap();

        // Load from disk
        let loaded = HeadIndex::load_from_disk(&path).unwrap();
        assert_eq!(loaded.heads.len(), 3);
        assert_eq!(
            loaded.get_head(&node_id1, "type1").unwrap(),
            Some(frame_id1)
        );
        assert_eq!(
            loaded.get_head(&node_id1, "type2").unwrap(),
            Some(frame_id2)
        );
        assert_eq!(
            loaded.get_head(&node_id2, "type1").unwrap(),
            Some(frame_id3)
        );
    }

    #[test]
    fn test_tombstone_and_restore_heads() {
        let mut index = HeadIndex::new();
        let node_id: NodeID = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];
        index.update_head(&node_id, "test", &frame_id).unwrap();
        assert_eq!(index.get_head(&node_id, "test").unwrap(), Some(frame_id));
        index.tombstone_heads_for_node(&node_id);
        assert_eq!(index.get_head(&node_id, "test").unwrap(), None);
        assert_eq!(index.get_active_head(&node_id, "test").unwrap(), None);
        index.restore_heads_for_node(&node_id);
        assert_eq!(index.get_head(&node_id, "test").unwrap(), Some(frame_id));
    }

    #[test]
    fn test_tombstone_single_head_keeps_other_frame_types_active() {
        let mut index = HeadIndex::new();
        let node_id: NodeID = [1u8; 32];
        let final_frame_id: FrameID = [2u8; 32];
        let intermediate_frame_id: FrameID = [3u8; 32];

        index
            .update_head(&node_id, "context-docs-writer", &final_frame_id)
            .unwrap();
        index
            .update_head(
                &node_id,
                "context-docs-writer--workflow-turn-1",
                &intermediate_frame_id,
            )
            .unwrap();

        let tombstoned = index.tombstone_head(&node_id, "context-docs-writer");

        assert_eq!(tombstoned, Some(final_frame_id));
        assert_eq!(
            index.get_head(&node_id, "context-docs-writer").unwrap(),
            None
        );
        assert_eq!(
            index
                .get_head(&node_id, "context-docs-writer--workflow-turn-1")
                .unwrap(),
            Some(intermediate_frame_id)
        );
    }

    #[test]
    fn test_purge_tombstoned() {
        let mut index = HeadIndex::new();
        let node_id: NodeID = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];
        index.update_head(&node_id, "test", &frame_id).unwrap();
        index.tombstone_heads_for_node(&node_id);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        index.purge_tombstoned(ts);
        assert_eq!(index.heads.len(), 0);
    }
}
