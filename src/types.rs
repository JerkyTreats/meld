//! Core types for the Merkle filesystem state management system.

/// NodeID: Deterministic hash of a filesystem node (file or directory)
pub type NodeID = [u8; 32];

/// FrameID: Deterministic hash of a context frame
pub type FrameID = [u8; 32];

/// Hash: Generic 256-bit hash value
pub type Hash = [u8; 32];
