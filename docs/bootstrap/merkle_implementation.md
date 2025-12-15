# Merkle Implementation Specification

## Overview

This document specifies the Merkle tree and Merkle set implementations for the filesystem state management system. The implementation uses a hybrid approach: custom N-ary trees for filesystem representation and sorted binary Merkle trees for frame sets.

---

## Design Decisions

### Filesystem Merkle Tree: N-ary Tree

**Decision**: Custom N-ary Merkle tree with variable fanout matching directory structure.

**Rationale**:
- Natural mapping to filesystem structure (directories have N children)
- No padding required for non-power-of-2 sizes
- Efficient for directories with varying child counts
- Simpler than Sparse Merkle Trees or Merkle Patricia Trees for this use case

**Structure**:
```
Directory Node:
  hash(
    node_type: "directory",
    path: normalized_path_bytes,
    children: [
      (name₁, hash₁),
      (name₂, hash₂),
      ...
      (nameₙ, hashₙ)  // sorted lexicographically
    ],
    metadata: {...}
  ) → NodeID

File Node:
  hash(
    node_type: "file",
    path: normalized_path_bytes,
    content_hash: hash(file_bytes),
    metadata: {size, permissions, ...}
  ) → NodeID
```

**Key Properties**:
- **Deterministic Ordering**: Children sorted by name (lexicographic, case-sensitive)
- **Content Hashing**: Files hashed by content, not path
- **Metadata Inclusion**: Non-content metadata included in hash
- **Lossless**: Full tree structure recoverable from root hash + node store

### Frame Set Merkle Tree: Sorted Binary Tree

**Decision**: Sorted binary Merkle tree over sorted frame list using `rs-merkle` crate with BLAKE3 adapter.

**Rationale**:
- Simple to implement and verify
- Deterministic: same set → same root (regardless of insertion order)
- Efficient for bounded sets (typically < 1000 frames)
- Supports membership proofs (future enhancement)
- Production-tested library (`rs-merkle`) with custom hash adapter

**Algorithm**:
1. Sort frames deterministically (lexicographic sort of `[u8; 32]`)
2. Build binary Merkle tree over sorted list
3. Each leaf = `hash("frame_leaf" || FrameID)`
4. Each internal node = `hash(left_child || right_child)`

**Empty Set**: Stable empty set root hash (constant `EMPTY_SET_HASH`)

### Hash Function: BLAKE3

**Decision**: BLAKE3 as primary hash function.

**Rationale**:
- Very fast (SIMD-optimized, parallelizable)
- Tree hashing built-in
- 256-bit output (good security margin)
- No known vulnerabilities

**Fallback**: SHA-256 available as alternative if needed.

---

## Implementation Architecture

### Filesystem Merkle Tree (Custom N-ary)

**Data Structures**:
```rust
pub type NodeID = [u8; 32];
pub type Hash = [u8; 32];

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub content_hash: Hash,
    pub size: u64,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub path: PathBuf,
    pub children: Vec<(String, NodeID)>, // (name, node_id) sorted by name
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum MerkleNode {
    File(FileNode),
    Directory(DirectoryNode),
}
```

**NodeID Computation**:
- **File NodeID**: `hash("file" || path_len || path || content_hash || metadata)`
- **Directory NodeID**: `hash("directory" || path_len || path || children_count || children || metadata)`

**Incremental Updates**:
- Track changed nodes only
- Recompute hashes bottom-up from changed leaves
- Complexity: O(d) where d = depth of changed node
- Batch updates: O(d × m) where m = number of changed files

### Frame Set Merkle Tree (rs-merkle with BLAKE3)

**Data Structures**:
```rust
pub type FrameID = [u8; 32];

/// BLAKE3 algorithm adapter for rs-merkle
#[derive(Clone, Debug)]
pub struct Blake3Algorithm;

impl Algorithm<[u8; 32]> for Blake3Algorithm {
    fn hash(&self, left: &[u8; 32], right: &[u8; 32]) -> [u8; 32];
    fn hash_leaf(&self, leaf: &[u8; 32]) -> [u8; 32];
}

pub struct FrameMerkleSet {
    frames: BTreeSet<FrameID>, // Sorted set
    tree: Option<MerkleTree<Blake3Algorithm>>,
    root: Option<[u8; 32]>,
}
```

**Operations**:
- `add_frame(frame_id)`: Insert frame, rebuild tree, return root
- `remove_frame(frame_id)`: Remove frame, rebuild tree, return root
- `root()`: Get current root hash
- `prove_membership(frame_id)`: Generate membership proof (future)
- `verify_membership(frame_id, proof)`: Verify membership proof (future)

**Update Strategy**:
- Phase 1: Full rebuild on each change (O(n log n))
- Phase 2: Incremental updates (O(log n)) - maintain tree structure

---

## Performance Optimizations

### Incremental Tree Updates

**Filesystem Tree**:
- Update only changed nodes
- Propagate hash changes up tree from leaf to root
- Cache intermediate hashes
- Batch multiple file changes in single pass

**Frame Set**:
- Maintain sorted set structure
- For Phase 1: Full rebuild (simple, acceptable for bounded sets)
- For Phase 2: Incremental path updates (O(log n))

### Parallel Hashing

**Strategy**:
- Hash files in parallel (thread pool)
- Hash directory children in parallel
- Use BLAKE3's built-in parallel hashing
- Implementation: `rayon` crate for parallel iteration

### Caching Strategy

- **Node Hash Cache**: Cache computed node hashes until invalidation
- **Frame Set Cache**: Cache frame set roots
- **Invalidation**: Hash-based (if parent hash changes, invalidate cache)

---

## Lossless Guarantees

### Definition

A lossless Merkle implementation ensures:
1. **Full Reconstruction**: Given root hash + all node data, can reconstruct exact tree
2. **No Information Loss**: All structural and content information preserved
3. **Deterministic**: Same input → same root (always)
4. **Verifiable**: Can verify any node's inclusion without full tree

### Filesystem Tree Guarantees

- **Content**: File contents fully preserved (content-addressed)
- **Structure**: Directory structure fully preserved (children list)
- **Metadata**: Selected metadata preserved (size, type)
- **Lossless**: Can reconstruct exact filesystem from root + node store

### Frame Set Guarantees

- **Membership**: All frames preserved (no frames lost)
- **Ordering**: Deterministic ordering preserved
- **Content**: Frame content preserved (content-addressed)
- **Lossless**: Can reconstruct exact frame set from root + frame store

### Verification

- **Tree Verification**: Given root hash, can verify any node's inclusion
- **Set Verification**: Given set root, can verify any frame's membership
- **Consistency**: Can verify stored roots match recomputed roots

---

## Edge Cases & Handling

### Empty Structures

- **Empty Directory**: `hash("directory" || path || [])` → stable hash
- **Empty Frame Set**: Constant `EMPTY_SET_HASH` (documented)
- **Single Child**: Normal n-ary tree, no special case needed

### Collision Handling

- **Hash Collisions**: Extremely unlikely with 256-bit hashes (2^256 space)
- **Mitigation**: Use cryptographic hash (BLAKE3), error on collision detection
- **Path Collisions**: Include path in hash (already done)

### Large Files

- **Memory Efficiency**: Stream hashing (BLAKE3 supports this)
- **Implementation**: Use `blake3::Hasher` with buffered reads (8KB chunks)

### Concurrent Updates

- **Problem**: Multiple processes updating tree simultaneously
- **Solution**: File locking (flock) or database transactions, detect conflicts via hash comparison

### Symlink Handling

- **Decision**: Follow symlinks (canonicalize) for Phase 1
- **Implementation**: Use `dunce::canonicalize()` for cross-platform support

### Unicode & Normalization

- **Problem**: Different Unicode representations of same character
- **Solution**: NFC normalization before hashing
- **Implementation**: Use `unicode-normalization` crate

### Platform Differences

- **Case Sensitivity**: Unix case-sensitive, Windows case-insensitive
- **Path Separators**: `/` vs `\`
- **Solution**: Normalize paths to canonical form, use `PathBuf` and `dunce`

---

## Testing Requirements

### Correctness Tests

- **Determinism**: Same input → same output (property tests)
- **Losslessness**: Reconstruct tree from root + data
- **Incremental Updates**: Update matches full rebuild
- **Edge Cases**: 
  - Empty trees, single node, large trees
  - Empty directories, empty files
  - Symlinks, hard links
  - Unicode filenames, special characters
  - Very deep directory structures
  - Very wide directories (many children)

### Performance Tests

- **Tree Construction**: Time for various tree sizes (10, 100, 1000, 10000 nodes)
- **Incremental Updates**: Time for single node update
- **Set Operations**: Time for frame addition/removal
- **Parallel Speedup**: Measure parallel hashing improvement
- **Large Files**: Hashing performance for files (1MB, 10MB, 100MB, 1GB)

### Property-Based Tests

- Use `proptest` to generate random filesystem structures
- Verify determinism, losslessness, incremental correctness
- Test with various directory depths, file sizes, Unicode characters, special filenames

### Performance Targets

| Operation | Target | Acceptable | Excellent |
|-----------|--------|------------|-----------|
| File NodeID (small file) | < 1ms | < 100µs | < 10µs |
| Directory NodeID (100 children) | < 1ms | < 500µs | < 100µs |
| Frame set add (100 frames) | < 10ms | < 5ms | < 1ms |
| Frame set rebuild (1000 frames) | < 50ms | < 20ms | < 10ms |
| Large file hash (100MB) | < 500ms | < 200ms | < 100ms |

---

## Dependencies

### Required Crates

```toml
[dependencies]
# Hashing
blake3 = "1.5"

# Merkle tree for frame sets
rs-merkle = "2.0"  # or merkletree = "0.24" as alternative

# Serialization
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"

# Storage
sled = "0.34"  # or rocksdb = "0.21"

# Filesystem
walkdir = "2.4"
dunce = "1.0"
unicode-normalization = "0.1"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Concurrency
parking_lot = "0.12"
rayon = "1.8"  # For parallel hashing

[dev-dependencies]
# Testing
proptest = "1.4"
tempfile = "3.8"

# Benchmarking
criterion = "0.5"
```

**Note**: The `rs-merkle` API may vary by version. Verify the exact API in the crate documentation before implementation. If `rs-merkle` doesn't support the required interface, `merkletree` is a good alternative with similar features.

---

## Implementation Phases

### Phase 1: Core Algorithms
1. Implement custom N-ary filesystem Merkle tree
2. Implement frame set Merkle tree with `rs-merkle`
3. Add BLAKE3 adapters
4. Basic unit tests

### Phase 2: Incremental Updates
1. Implement incremental tree updates
2. Optimize frame set updates
3. Add caching layer
4. Performance benchmarks

### Phase 3: Verification & Optimization
1. Add proof generation/verification
2. Property-based tests
3. Integration tests
4. Performance tuning

---

## Future Enhancements

### Membership Proofs

**Structure**:
```rust
struct MembershipProof {
    frame_id: FrameID,
    path: Vec<ProofNode>,  // Sibling hashes from leaf to root
}

struct ProofNode {
    hash: Hash,
    is_left: bool,  // Position relative to path
}
```

**Properties**:
- **Proof Size**: O(log n) where n = number of frames
- **Verification**: O(log n) to verify proof
- **Non-Membership Proofs**: Possible with sorted structure

### Incremental Frame Set Updates

- Maintain explicit tree structure with parent pointers
- Update only affected path from leaf to root
- Cache sibling hashes for efficient updates
- Complexity: O(log n) for insertion

---

[← Back to Components](phase1_components.md) | [Next: Implementation →](phase1_implementation.md)

