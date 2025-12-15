# Phase 1 Implementation Guide

## Technology Stack
- **Language**: Rust
- **Edition**: 2021 or later
- **Minimum Rust Version**: 1.70+

---

## Hash Algorithm Selection
- **Recommendation**: `blake3` crate (BLAKE3) or `sha2` crate (SHA-256)
- **Primary Choice**: BLAKE3 for performance (faster, parallelizable)
- **Requirements**: Cryptographic hash, deterministic, fast
- **Considerations**: 
  - BLAKE3: Better performance, SIMD-optimized, tree hashing support
  - SHA-256: More widely used, standard library support
- **Implementation**: Use `[u8; 32]` for hash values (256 bits)

---

## Path Normalization Rules
- **Crate**: `std::path::PathBuf` with `dunce` crate for canonicalization
- **Rules**:
  - Resolve `..` and `.` components using `PathBuf::canonicalize()` or `dunce::canonicalize()`
  - Remove trailing slashes (except root)
  - Handle symlinks: Use `std::fs::canonicalize()` or `dunce::canonicalize()` (cross-platform)
  - Case sensitivity: Platform-specific (Unix case-sensitive, Windows case-insensitive)
  - Unicode normalization: Use `unicode-normalization` crate for NFC normalization
- **Path Types**: Use `PathBuf` for owned paths, `&Path` for borrowed

---

## Storage Backend Options

### NodeRecord Store
- **Options**:
  - `rocksdb` crate: Embedded key-value store, persistent, high performance
  - `sled` crate: Pure Rust embedded database, ACID transactions
  - `HashMap<[u8; 32], NodeRecord>`: In-memory only (for testing/development)
- **Recommendation**: Start with `sled` for simplicity, migrate to `rocksdb` if needed
- **Key Type**: `[u8; 32]` (NodeID as fixed-size array)
- **Value Type**: Serialized `NodeRecord` (see Serialization)

### Frame Storage
- **Options**:
  - Filesystem: `std::fs` with content-addressed paths (`frames/{FrameID[0..2]}/{FrameID[2..4]}/{FrameID}`)
  - `sled` or `rocksdb`: Blob storage in database
- **Recommendation**: Filesystem for simplicity and content-addressability
- **Path Structure**: `{store_root}/frames/{hex_prefix}/{FrameID}.frame`

### Head Index
- **Data Structure**: `HashMap<(NodeID, FrameType), FrameID>` or `BTreeMap<(NodeID, FrameType), FrameID>`
- **Persistence**: Serialize to disk periodically or use `sled` for persistence
- **Key Type**: `([u8; 32], String)` or custom tuple type

---

## Data Structures (Rust)

### Core Types
```rust
type NodeID = [u8; 32];
type FrameID = [u8; 32];
type Hash = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeRecord {
    node_id: NodeID,
    path: PathBuf,
    node_type: NodeType, // File | Directory
    children: Vec<NodeID>,
    parent: Option<NodeID>,
    frame_set_root: Option<Hash>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum NodeType {
    File { size: u64, content_hash: Hash },
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Frame {
    frame_id: FrameID,
    basis: Basis, // NodeID | FrameID | Both
    content: Vec<u8>, // Blob
    metadata: HashMap<String, String>, // Non-hashed
    timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Basis {
    Node(NodeID),
    Frame(FrameID),
    Both { node: NodeID, frame: FrameID },
}
```

---

## Serialization
- **Crate**: `serde` with `serde_json` or `bincode`
- **Recommendation**: `bincode` for binary serialization (smaller, faster)
- **Format**: Binary for storage, JSON for debugging/logging
- **Traits**: Derive `Serialize` and `Deserialize` for all data structures

---

## Error Handling
- **Crate**: `thiserror` for error types, `anyhow` for application-level errors
- **Pattern**: Use `Result<T, E>` throughout, avoid panics in library code
- **Error Types**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  enum MerkleError {
      #[error("Invalid NodeID: {0:?}")]
      InvalidNodeID([u8; 32]),
      #[error("Frame not found: {0:?}")]
      FrameNotFound(FrameID),
      #[error("Hash mismatch: expected {expected:?}, got {actual:?}")]
      HashMismatch { expected: Hash, actual: Hash },
      // ...
  }
  ```
- **Graceful Degradation**: Return `Option<T>` or `Result<T, E>` rather than panicking

---

## Concurrency
- **Patterns**:
  - `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared mutable state
  - `Arc<T>` for shared immutable data
  - Channels (`std::sync::mpsc` or `tokio::sync::mpsc`) for message passing
- **Recommendation**: 
  - Use `RwLock` for read-heavy workloads (NodeRecord Store)
  - Use `Mutex` for write-heavy or simple cases
  - Consider `parking_lot` crate for better performance
- **Atomic Updates**: Use transactions where possible (e.g., `sled::Tree::transaction`)

---

## Filesystem Operations
- **Crate**: `std::fs` for basic operations, `walkdir` crate for directory traversal
- **Walker**: Use `walkdir::WalkDir` for filesystem traversal
- **Async**: Consider `tokio::fs` if async I/O is needed
- **Symlinks**: Handle via `walkdir` options or `std::fs::read_link()`

---

## Performance Targets
- NodeID lookup: < 1ms
- Frame retrieval: < 10ms for bounded view
- Tree recomputation: O(n) where n = number of changed nodes
- Context view construction: O(m log m) where m = frame count (bounded)

---

## Testing Strategy
- **Unit Tests**: `#[cfg(test)]` modules in each file
- **Integration Tests**: `tests/` directory
- **Property-Based Testing**: `proptest` crate for determinism and fuzzing
- **Benchmarks**: `criterion` crate for performance benchmarks
- **Test Data**: Use `tempfile` crate for temporary directories
- **Example**:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use proptest::prelude::*;
      
      proptest! {
          #[test]
          fn test_deterministic_hashing(data in any::<Vec<u8>>()) {
              let hash1 = compute_hash(&data);
              let hash2 = compute_hash(&data);
              prop_assert_eq!(hash1, hash2);
          }
      }
  }
  ```

---

## Project Structure
```
merkle/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tree/
│   │   ├── mod.rs          # Filesystem Merkle Tree
│   │   ├── node.rs
│   │   └── hasher.rs
│   ├── store/
│   │   ├── mod.rs          # NodeRecord Store
│   │   └── persistence.rs
│   ├── frame/
│   │   ├── mod.rs          # Context Frames
│   │   ├── storage.rs
│   │   └── set.rs          # Frame Merkle Set
│   ├── heads.rs            # Frame Heads
│   ├── views.rs            # Context Views
│   ├── types.rs            # Core types (NodeID, FrameID, etc.)
│   └── error.rs            # Error types
├── tests/
│   ├── integration/
│   └── property/
└── benches/                # Criterion benchmarks
```

---

## Dependencies (Cargo.toml)
```toml
[dependencies]
blake3 = "1.5"              # or sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"             # or serde_json = "1.0"
sled = "0.34"               # or rocksdb = "0.21"
walkdir = "2.4"
dunce = "1.0"
unicode-normalization = "0.1"
thiserror = "1.0"
anyhow = "1.0"
parking_lot = "0.12"        # Optional: better locks

[dev-dependencies]
proptest = "1.4"
criterion = "0.5"
tempfile = "3.8"

[[bench]]
name = "node_lookup"
harness = false
```

---

[← Back to Merkle Implementation](merkle_implementation.md) | [Back to Spec](phase1_spec.md)

