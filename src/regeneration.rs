//! Incremental Regeneration
//!
//! Rebuilds derived context frames when their basis changes. Regeneration is incremental,
//! localized, and basis-driven—only frames whose basis has changed are regenerated.
//! Old frames are retained (append-only), ensuring full history preservation.

use crate::error::ApiError;
use crate::frame::{Basis, Frame, FrameStorage};
use crate::frame::id::compute_basis_hash;
use crate::heads::HeadIndex;
use crate::store::NodeRecordStore;
use crate::synthesis::{collect_child_frames, synthesize_content, SynthesisBasis, SynthesisPolicy};
use crate::types::{FrameID, Hash, NodeID};
use std::collections::HashMap;

/// Basis index: basis_hash → Vec<FrameID>
///
/// Enables fast lookup of frames affected by basis changes.
/// Maps a basis hash to all frames that were created with that basis.
pub struct BasisIndex {
    /// Index: basis_hash → Vec<FrameID>
    index: HashMap<Hash, Vec<FrameID>>,
    /// Reverse index: FrameID → basis_hash (for cleanup)
    reverse: HashMap<FrameID, Hash>,
}

impl Default for BasisIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl BasisIndex {
    /// Create a new empty basis index
    pub fn new() -> Self {
        BasisIndex {
            index: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Add a frame to the index
    ///
    /// Associates the frame's basis hash with its FrameID.
    pub fn add_frame(&mut self, basis_hash: Hash, frame_id: FrameID) {
        self.index
            .entry(basis_hash)
            .or_insert_with(Vec::new)
            .push(frame_id);
        self.reverse.insert(frame_id, basis_hash);
    }

    /// Remove a frame from the index
    ///
    /// Note: This doesn't delete the frame from storage, just removes it from the index.
    /// Old frames are preserved (append-only).
    pub fn remove_frame(&mut self, frame_id: &FrameID) {
        if let Some(basis_hash) = self.reverse.remove(frame_id) {
            if let Some(frame_ids) = self.index.get_mut(&basis_hash) {
                frame_ids.retain(|&id| id != *frame_id);
                if frame_ids.is_empty() {
                    self.index.remove(&basis_hash);
                }
            }
        }
    }

    /// Get all frames with a given basis hash
    pub fn get_frames_by_basis(&self, basis_hash: &Hash) -> Vec<FrameID> {
        self.index
            .get(basis_hash)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Get the basis hash for a frame
    pub fn get_basis_for_frame(&self, frame_id: &FrameID) -> Option<Hash> {
        self.reverse.get(frame_id).copied()
    }

    /// Check if a basis hash exists in the index
    pub fn has_basis(&self, basis_hash: &Hash) -> bool {
        self.index.contains_key(basis_hash)
    }

    /// Get the number of basis entries in the index
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Iterate over all basis entries
    pub fn iter(&self) -> impl Iterator<Item = (&Hash, &Vec<FrameID>)> {
        self.index.iter()
    }
}

/// Regeneration report
///
/// Summary of regeneration results.
#[derive(Debug, Clone)]
pub struct RegenerationReport {
    /// NodeID that was regenerated
    pub node_id: NodeID,
    /// Number of frames regenerated
    pub regenerated_count: usize,
    /// FrameIDs of regenerated frames
    pub frame_ids: Vec<FrameID>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Detect basis changes for a node
///
/// Compares the stored basis hash for each frame type with the current basis hash.
/// For synthesized frames, compares the synthesis basis hash (from metadata).
/// Returns a list of frame types that need regeneration.
pub fn detect_basis_changes(
    node_id: NodeID,
    frame_types: &[String],
    basis_index: &BasisIndex,
    head_index: &HeadIndex,
    frame_storage: &FrameStorage,
    node_store: &dyn NodeRecordStore,
) -> Result<Vec<String>, ApiError> {
    let mut changed_types = Vec::new();

    for frame_type in frame_types {
        // Get current head frame for this type
        let head_frame_id = match head_index.get_head(&node_id, frame_type).map_err(ApiError::from)? {
            Some(id) => id,
            None => continue, // No frame to regenerate
        };

        // Get the frame to determine its basis type
        let frame = match frame_storage.get(&head_frame_id).map_err(ApiError::from)? {
            Some(f) => f,
            None => continue, // Frame not found, skip
        };

        // Check if this is a synthesized frame
        if frame.metadata.contains_key("basis_hash") {
            // Synthesized frame - compare synthesis basis hash
            let stored_basis_hash_str = match frame.metadata.get("basis_hash") {
                Some(s) => s,
                None => continue, // Missing basis_hash in metadata, skip
            };

            // Parse stored basis hash from hex string
            let stored_basis_hash = match hex_string_to_hash(stored_basis_hash_str) {
                Ok(hash) => hash,
                Err(_) => continue, // Invalid hash format, skip
            };

            // Get current child frames and compute current synthesis basis
            // We need to clone head_index data or use a different approach
            // For now, we'll collect child frames directly
            let node_record = node_store
                .get(&node_id)
                .map_err(ApiError::from)?
                .ok_or_else(|| ApiError::NodeNotFound(node_id))?;

            let mut child_frames: Vec<(NodeID, Frame)> = Vec::new();
            for child_node_id in &node_record.children {
                if let Some(head_frame_id) = head_index.get_head(child_node_id, frame_type).map_err(ApiError::from)? {
                    if let Some(frame) = frame_storage.get(&head_frame_id).map_err(ApiError::from)? {
                        child_frames.push((*child_node_id, frame));
                    }
                }
            }

            // Sort deterministically
            child_frames.sort_by(|(node_id_a, frame_a), (node_id_b, frame_b)| {
                node_id_a.cmp(node_id_b).then_with(|| {
                    frame_a.frame_id.cmp(&frame_b.frame_id)
                })
            });

            // Extract synthesis policy
            let policy = if let Some(policy_str) = frame.metadata.get("synthesis_policy") {
                if policy_str == "concatenation" || policy_str == "Concatenation" {
                    SynthesisPolicy::Concatenation
                } else if policy_str == "summarization" || policy_str == "Summarization" {
                    SynthesisPolicy::Summarization
                } else {
                    SynthesisPolicy::Concatenation // Default
                }
            } else {
                SynthesisPolicy::Concatenation
            };

            // Compute current synthesis basis hash
            let child_frame_ids: Vec<FrameID> = child_frames.iter().map(|(_, frame)| frame.frame_id).collect();
            let current_basis_info = SynthesisBasis {
                node_id,
                child_frame_ids,
                frame_type: frame_type.clone(),
                synthesis_policy: policy,
            };
            let current_basis_hash = current_basis_info.compute_hash();

            // Check if basis has changed
            if stored_basis_hash != current_basis_hash {
                changed_types.push(frame_type.clone());
            }
        } else {
            // Regular frame - compare frame basis hash
            let stored_basis_hash = match basis_index.get_basis_for_frame(&head_frame_id) {
                Some(hash) => hash,
                None => continue, // Frame not in index, skip
            };

            // Compute current basis hash
            let current_basis_hash = compute_basis_hash(&frame.basis).map_err(ApiError::from)?;

            // Check if basis has changed
            if stored_basis_hash != current_basis_hash {
                changed_types.push(frame_type.clone());
            }
        }
    }

    Ok(changed_types)
}

/// Convert hex string to Hash
fn hex_string_to_hash(hex: &str) -> Result<Hash, ApiError> {
    if hex.len() != 64 {
        return Err(ApiError::InvalidFrame(format!("Invalid hash length: {}", hex.len())));
    }

    let mut hash = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        if i >= 32 {
            break;
        }
        let byte_str = std::str::from_utf8(chunk)
            .map_err(|e| ApiError::InvalidFrame(format!("Invalid hex string: {}", e)))?;
        hash[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|e| ApiError::InvalidFrame(format!("Invalid hex digit: {}", e)))?;
    }

    Ok(hash)
}

/// Regenerate frames for a node
///
/// Regenerates all frames whose basis has changed. For synthesized frames,
/// this will re-synthesize from current child frames.
pub fn regenerate_node(
    node_id: NodeID,
    recursive: bool,
    basis_index: &mut BasisIndex,
    head_index: &mut HeadIndex,
    frame_storage: &FrameStorage,
    node_store: &dyn NodeRecordStore,
    agent_id: String,
) -> Result<RegenerationReport, ApiError> {
    let start_time = std::time::Instant::now();

    // Get all frame types for this node
    let all_frame_ids = head_index.get_all_heads_for_node(&node_id);
    let mut frame_types: Vec<String> = Vec::new();

    // Extract frame types from head index
    // We need to get frame types from the frames themselves
    for frame_id in &all_frame_ids {
        if let Some(frame) = frame_storage.get(frame_id).map_err(ApiError::from)? {
            if !frame_types.contains(&frame.frame_type) {
                frame_types.push(frame.frame_type.clone());
            }
        }
    }

    // Detect basis changes
    let changed_types = detect_basis_changes(
        node_id,
        &frame_types,
        basis_index,
        head_index,
        frame_storage,
        node_store,
    )?;

    let mut regenerated_frame_ids = Vec::new();

    // Regenerate each changed frame type
    for frame_type in &changed_types {
        // Get current head frame
        let head_frame_id = match head_index.get_head(&node_id, frame_type).map_err(ApiError::from)? {
            Some(id) => id,
            None => continue,
        };

        let head_frame = match frame_storage.get(&head_frame_id).map_err(ApiError::from)? {
            Some(f) => f,
            None => continue,
        };

        // Check if this is a synthesized frame (has basis_hash in metadata)
        if head_frame.metadata.contains_key("basis_hash") {
            // This is a synthesized frame - re-synthesize from children
            let child_frames = collect_child_frames(
                node_store,
                frame_storage,
                head_index,
                node_id,
                frame_type,
            )?;

            if child_frames.is_empty() {
                // Empty directory - create empty frame
                let basis = Basis::Node(node_id);
                let content = b"Empty directory".to_vec();
                let metadata = {
                    let mut m = head_frame.metadata.clone();
                    m.insert("synthesis_policy".to_string(), "concatenation".to_string());
                    m
                };

                let new_frame = Frame::new(basis, content, frame_type.clone(), agent_id.clone(), metadata)?;

                // Store new frame
                frame_storage.store(&new_frame).map_err(ApiError::from)?;

                // Update basis index
                let basis_hash = compute_basis_hash(&new_frame.basis).map_err(ApiError::from)?;
                basis_index.add_frame(basis_hash, new_frame.frame_id);

                // Update head
                head_index.update_head(&node_id, frame_type, &new_frame.frame_id).map_err(ApiError::from)?;

                regenerated_frame_ids.push(new_frame.frame_id);
            } else {
                // Extract synthesis policy from metadata
                let policy = if let Some(policy_str) = head_frame.metadata.get("synthesis_policy") {
                    // Try to parse policy from string
                    if policy_str == "concatenation" {
                        SynthesisPolicy::Concatenation
                    } else if policy_str == "summarization" {
                        SynthesisPolicy::Summarization
                    } else {
                        SynthesisPolicy::Concatenation // Default
                    }
                } else {
                    SynthesisPolicy::Concatenation
                };

                // Extract child frame IDs
                let child_frame_ids: Vec<FrameID> = child_frames.iter().map(|(_, frame)| frame.frame_id).collect();

                // Construct synthesis basis
                let basis_info = SynthesisBasis {
                    node_id,
                    child_frame_ids: child_frame_ids.clone(),
                    frame_type: frame_type.clone(),
                    synthesis_policy: policy.clone(),
                };

                let basis_hash = basis_info.compute_hash();

                // Synthesize content
                let synthesized_content = synthesize_content(&child_frames, &policy);

                // Create basis
                let basis = if child_frame_ids.len() == 1 {
                    Basis::Frame(child_frame_ids[0])
                } else {
                    Basis::Node(node_id)
                };

                // Create frame metadata
                let mut metadata = head_frame.metadata.clone();
                metadata.insert("synthesis_policy".to_string(), format!("{:?}", policy));
                let basis_hash_hex: String = basis_hash.iter().map(|b| format!("{:02x}", b)).collect();
                metadata.insert("basis_hash".to_string(), basis_hash_hex);
                metadata.insert("child_frame_count".to_string(), child_frame_ids.len().to_string());

                // Create new synthesized frame
                let new_frame = Frame::new(basis, synthesized_content, frame_type.clone(), agent_id.clone(), metadata)?;

                // Store new frame
                frame_storage.store(&new_frame).map_err(ApiError::from)?;

                // Update basis index
                let frame_basis_hash = compute_basis_hash(&new_frame.basis).map_err(ApiError::from)?;
                basis_index.add_frame(frame_basis_hash, new_frame.frame_id);

                // Update head atomically
                head_index.update_head(&node_id, frame_type, &new_frame.frame_id).map_err(ApiError::from)?;

                regenerated_frame_ids.push(new_frame.frame_id);
            }
        } else {
            // Regular frame - check if basis changed
            let stored_basis_hash = match basis_index.get_basis_for_frame(&head_frame_id) {
                Some(hash) => hash,
                None => continue, // Not in index, skip
            };

            let current_basis_hash = compute_basis_hash(&head_frame.basis).map_err(ApiError::from)?;

            if stored_basis_hash != current_basis_hash {
                // Basis changed - need to regenerate
                // For non-synthesized frames, we can't automatically regenerate
                // as we don't know how to recreate the content.
                // This is a limitation - in practice, these frames would need
                // to be regenerated by the agent that created them.
                // For now, we'll skip them (they were already detected in detect_basis_changes
                // but we can't regenerate them automatically).
                continue;
            }
        }
    }

    // If recursive, regenerate child nodes
    if recursive {
        let node_record = node_store
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NodeNotFound(node_id))?;

        for child_node_id in &node_record.children {
            let child_report = regenerate_node(
                *child_node_id,
                true,
                basis_index,
                head_index,
                frame_storage,
                node_store,
                agent_id.clone(),
            )?;

            regenerated_frame_ids.extend(child_report.frame_ids);
        }
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;

    Ok(RegenerationReport {
        node_id,
        regenerated_count: regenerated_frame_ids.len(),
        frame_ids: regenerated_frame_ids,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Basis, Frame};
    use crate::frame::storage::FrameStorage;
    use crate::heads::HeadIndex;
    use crate::store::{NodeRecord, NodeRecordStore, NodeType, SledNodeRecordStore};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_basis_index_add_and_get() {
        let mut index = BasisIndex::new();
        let basis_hash: Hash = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];

        index.add_frame(basis_hash, frame_id);

        let frames = index.get_frames_by_basis(&basis_hash);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], frame_id);
    }

    #[test]
    fn test_basis_index_multiple_frames() {
        let mut index = BasisIndex::new();
        let basis_hash: Hash = [1u8; 32];
        let frame_id1: FrameID = [2u8; 32];
        let frame_id2: FrameID = [3u8; 32];

        index.add_frame(basis_hash, frame_id1);
        index.add_frame(basis_hash, frame_id2);

        let frames = index.get_frames_by_basis(&basis_hash);
        assert_eq!(frames.len(), 2);
        assert!(frames.contains(&frame_id1));
        assert!(frames.contains(&frame_id2));
    }

    #[test]
    fn test_basis_index_remove_frame() {
        let mut index = BasisIndex::new();
        let basis_hash: Hash = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];

        index.add_frame(basis_hash, frame_id);
        assert_eq!(index.get_frames_by_basis(&basis_hash).len(), 1);

        index.remove_frame(&frame_id);
        assert_eq!(index.get_frames_by_basis(&basis_hash).len(), 0);
    }

    #[test]
    fn test_basis_index_get_basis_for_frame() {
        let mut index = BasisIndex::new();
        let basis_hash: Hash = [1u8; 32];
        let frame_id: FrameID = [2u8; 32];

        index.add_frame(basis_hash, frame_id);

        let retrieved_hash = index.get_basis_for_frame(&frame_id);
        assert_eq!(retrieved_hash, Some(basis_hash));
    }

    #[test]
    fn test_hex_string_to_hash() {
        let hex = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let hash = hex_string_to_hash(hex).unwrap();
        assert_eq!(hash[0], 0x00);
        assert_eq!(hash[1], 0x01);
        assert_eq!(hash[31], 0x1f);
    }

    #[test]
    fn test_hex_string_to_hash_invalid_length() {
        let hex = "invalid";
        let result = hex_string_to_hash(hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_regeneration_idempotent() {
        // Test that regenerating the same node twice produces the same result
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let frame_storage_path = temp_dir.path().join("frames");

        let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(&frame_storage_path).unwrap());
        let mut head_index = HeadIndex::new();
        let mut basis_index = BasisIndex::new();

        // Create a directory node with a child
        let dir_node_id: NodeID = [1u8; 32];
        let child_node_id: NodeID = [2u8; 32];

        let child_record = NodeRecord {
            node_id: child_node_id,
            path: PathBuf::from("/test/child.txt"),
            node_type: NodeType::File {
                size: 100,
                content_hash: [0u8; 32],
            },
            children: vec![],
            parent: Some(dir_node_id),
            frame_set_root: None,
            metadata: HashMap::new(),
        };

        let dir_record = NodeRecord {
            node_id: dir_node_id,
            path: PathBuf::from("/test"),
            node_type: NodeType::Directory,
            children: vec![child_node_id],
            parent: None,
            frame_set_root: None,
            metadata: HashMap::new(),
        };

        node_store.put(&child_record).unwrap();
        node_store.put(&dir_record).unwrap();

        // Create a frame for the child
        let child_basis = Basis::Node(child_node_id);
        let child_content = b"child content".to_vec();
        let child_frame = Frame::new(
            child_basis,
            child_content,
            "test".to_string(),
            "agent-1".to_string(),
            HashMap::new(),
        ).unwrap();

        frame_storage.store(&child_frame).unwrap();
        head_index.update_head(&child_node_id, "test", &child_frame.frame_id).unwrap();
        let child_basis_hash = compute_basis_hash(&child_frame.basis).unwrap();
        basis_index.add_frame(child_basis_hash, child_frame.frame_id);

        // Synthesize a frame for the directory
        let child_frames = vec![(child_node_id, child_frame.clone())];
        let child_frame_ids: Vec<FrameID> = child_frames.iter().map(|(_, f)| f.frame_id).collect();
        let synthesis_basis = SynthesisBasis {
            node_id: dir_node_id,
            child_frame_ids: child_frame_ids.clone(),
            frame_type: "test".to_string(),
            synthesis_policy: SynthesisPolicy::Concatenation,
        };
        let basis_hash = synthesis_basis.compute_hash();
        let synthesized_content = synthesize_content(&child_frames, &SynthesisPolicy::Concatenation);

        let mut metadata = HashMap::new();
        metadata.insert("synthesis_policy".to_string(), "concatenation".to_string());
        let basis_hash_hex: String = basis_hash.iter().map(|b| format!("{:02x}", b)).collect();
        metadata.insert("basis_hash".to_string(), basis_hash_hex);
        metadata.insert("child_frame_count".to_string(), "1".to_string());

        let dir_basis = Basis::Node(dir_node_id);
        let dir_frame = Frame::new(
            dir_basis,
            synthesized_content,
            "test".to_string(),
            "agent-1".to_string(),
            metadata,
        ).unwrap();

        frame_storage.store(&dir_frame).unwrap();
        head_index.update_head(&dir_node_id, "test", &dir_frame.frame_id).unwrap();
        let dir_basis_hash = compute_basis_hash(&dir_frame.basis).unwrap();
        basis_index.add_frame(dir_basis_hash, dir_frame.frame_id);

        // First regeneration - should detect no changes
        let frame_types = vec!["test".to_string()];
        let changed = detect_basis_changes(
            dir_node_id,
            &frame_types,
            &basis_index,
            &head_index,
            &frame_storage,
            node_store.as_ref(),
        ).unwrap();

        assert_eq!(changed.len(), 0, "No changes should be detected on first check");

        // Regenerate (should be idempotent)
        let report1 = regenerate_node(
            dir_node_id,
            false,
            &mut basis_index,
            &mut head_index,
            &frame_storage,
            node_store.as_ref(),
            "agent-1".to_string(),
        ).unwrap();

        assert_eq!(report1.regenerated_count, 0, "First regeneration should produce no changes");

        // Regenerate again (should still be idempotent)
        let report2 = regenerate_node(
            dir_node_id,
            false,
            &mut basis_index,
            &mut head_index,
            &frame_storage,
            node_store.as_ref(),
            "agent-1".to_string(),
        ).unwrap();

        assert_eq!(report2.regenerated_count, 0, "Second regeneration should also produce no changes");
    }
}
