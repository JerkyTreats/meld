# Phase 1 Spec — Bootstrap Core Components

## Overview

This specification documents Phase 1 of the Merkle-based filesystem state management system. The system provides a deterministic, hash-based foundation for tracking filesystem state and associated context.

## Table of Contents

- [Goals + Outcomes](#goals--outcomes)
- [Dependencies & Assumptions](#dependencies--assumptions)
- [Major Components](#major-components)
- [Component Relationships](#component-relationships)
- [API Specifications](#api-specifications)
- [Error Handling](#error-handling)
- [Performance Considerations](#performance-considerations)
- [Constraints & Non-Goals](#constraints--non-goals)
- [Development Phases](phase1_phases.md)
- [Phase Exit Criteria](#phase-exit-criteria)

---

## Goals + Outcomes

### Goals
- Establish a deterministic, Merkle-addressed foundation for filesystem state and context.
- Enable fast, scan-free traversal of nodes and attached context.
- Make context append-only, verifiable, and regenerable.
- Provide the minimal substrate required for agent read/write workflows.

### Outcomes
- Stable workspace root hash
- Stable NodeID and FrameID generation
- Bounded, deterministic context retrieval
- Hash-based invalidation only

---

## Dependencies & Assumptions

### Prerequisites
Phase 1 is the foundational phase and has no dependencies on other phases. It assumes:

- **Rust Toolchain**: Rust 1.70+ with standard library
- **Filesystem Access**: Read access to workspace filesystem
- **Storage Backend**: Persistent storage (filesystem or embedded database)
- **Hash Algorithm**: Cryptographic hash function (BLAKE3 or SHA-256)

### System Assumptions
- **Workspace Stability**: Workspace root path is stable during operation
- **Path Canonicalization**: Paths can be normalized deterministically
- **File Content Stability**: File content doesn't change during read operations
- **Storage Persistence**: Storage backend persists data reliably

### Design Constraints
- **Determinism First**: All operations must be deterministic
- **No External Dependencies**: Core engine has no network or external API calls
- **Single Workspace**: One workspace per engine instance
- **Append-Only**: All data structures are append-only (no mutation)

---

## Major Components

Phase 1 consists of six core components that provide the foundational substrate:

1. **Filesystem Merkle Tree**: Deterministic representation of filesystem structure
2. **NodeRecord Store**: Fast lookup storage for node metadata
3. **Context Frames**: Immutable, append-only context containers
4. **Context Frame Merkle Set**: Deterministic frame set membership
5. **Frame Heads**: Efficient pointers to latest frames
6. **Context Views**: Policy-driven frame selection

For detailed component specifications, see **[Component Specifications](phase1_components.md)**.

### Component Overview

#### Filesystem Merkle Tree
Represents the entire workspace as a Merkle tree. Each node (file or directory) has a deterministic hash based on content and structure. Changes propagate up the tree, invalidating ancestor hashes.

#### NodeRecord Store
Provides O(1) lookup of node metadata by NodeID. Stores structural relationships (parent, children) and pointers to associated frame sets. Acts as an index into the filesystem tree.

#### Context Frames
Immutable containers for context information associated with nodes. Each frame is content-addressed (FrameID = hash(content + basis)). Frames are append-only and never modified.

#### Context Frame Merkle Set
Maintains a deterministic set of frames for each node using a Merkle set structure. Enables efficient membership verification and set comparison through hash-based operations.

#### Frame Heads
Provides O(1) access to the "latest" frame for a given node and frame type. Enables fast access without scanning frame sets. Head pointers are updated when new frames are added.

#### Context Views
Selects and orders a bounded set of frames based on policies (recency, type, agent). Ensures deterministic, bounded context retrieval for agent consumption.

---

## Component Relationships

For detailed architecture and component relationships, see **[Architecture Overview](phase1_architecture.md)**.

### Data Flow
```
Filesystem Changes
    ↓
Filesystem Merkle Tree (recompute)
    ↓
NodeID Changes
    ↓
NodeRecord Store (update)
    ↓
Frame Set Invalidation
    ↓
Context Frame Merkle Set (update)
    ↓
Frame Heads (update)
    ↓
Context Views (select frames)
    ↓
Agent Consumption
```

### Dependencies
- **NodeRecord Store** depends on **Filesystem Merkle Tree** (for NodeIDs)
- **Context Frame Merkle Set** depends on **Context Frames** (contains FrameIDs)
- **Frame Heads** depend on **Context Frames** (points to FrameIDs)
- **Context Views** depend on **Frame Heads** and **Frame Sets** (for selection)

### System Properties
- **Determinism**: Same inputs → same outputs (hashes, IDs, sets)
- **Performance**: O(1) node lookup, O(1) head resolution, O(log n) set operations
- **Losslessness**: Full reconstruction possible from root + stores

---

## API Specifications

Phase 1 provides foundational APIs that Phase 2 will build upon. These are internal APIs used by the engine itself.

### Core Operations

#### Compute NodeID
```rust
fn compute_node_id(
    path: &Path,
    content: Option<&[u8]>,  // None for directories
    children: &[NodeID],
) -> NodeID;
```

Computes deterministic NodeID from path, content, and children.

#### Store NodeRecord
```rust
fn store_node_record(
    node_record: NodeRecord,
) -> Result<(), StorageError>;
```

Stores node metadata in NodeRecord Store.

#### Create Frame
```rust
fn create_frame(
    basis: Basis,
    content: Vec<u8>,
    metadata: HashMap<String, String>,
) -> Frame;
```

Creates a new immutable frame with deterministic FrameID.

#### Add Frame to Set
```rust
fn add_frame_to_set(
    node_id: NodeID,
    frame_id: FrameID,
) -> Result<Hash, StorageError>;  // Returns new set root
```

Adds frame to node's frame set, returns new set root hash.

#### Update Head
```rust
fn update_head(
    node_id: NodeID,
    frame_type: String,
    frame_id: FrameID,
) -> Result<(), StorageError>;
```

Updates head pointer for (node_id, frame_type) pair.

#### Get Context View
```rust
fn get_context_view(
    node_id: NodeID,
    policy: ViewPolicy,
) -> Result<Vec<FrameID>, StorageError>;
```

Retrieves frames for node according to view policy.

For implementation details, see **[Implementation Guide](phase1_implementation.md)**.

---

## Error Handling

### Error Types

#### StorageError
```rust
#[derive(Debug, thiserror::Error)]
enum StorageError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeID),
    
    #[error("Frame not found: {0:?}")]
    FrameNotFound(FrameID),
    
    #[error("Hash mismatch: expected {expected:?}, got {actual:?}")]
    HashMismatch { expected: Hash, actual: Hash },
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Storage I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
```

### Error Handling Principles
- **Deterministic**: Same error conditions → same error responses
- **No Panics**: All errors returned as Result types
- **Clear Messages**: Error messages include context (NodeID, FrameID, etc.)
- **Error Propagation**: Errors bubble up with context preserved

### Common Error Scenarios
- **Node Not Found**: NodeID doesn't exist in NodeRecord Store
- **Frame Not Found**: FrameID doesn't exist in frame storage
- **Hash Mismatch**: Computed hash doesn't match stored hash (corruption detection)
- **Invalid Path**: Path cannot be canonicalized or doesn't exist

For detailed error handling patterns, see **[Implementation Guide](phase1_implementation.md)**.

---

## Performance Considerations

### Performance Targets

#### Core Operations
- **NodeID computation**: < 1ms per node
- **NodeRecord lookup**: < 1ms (O(1) hash table)
- **Frame creation**: < 1ms (hash computation)
- **Frame set update**: < 10ms (O(log n) Merkle set)
- **Head resolution**: < 1ms (O(1) hash table)
- **Context view construction**: < 10ms for 100 frames

#### Tree Operations
- **Tree recomputation**: O(n) where n = number of changed nodes
- **Root hash computation**: O(d) where d = tree depth
- **Node traversal**: O(n) for full tree walk

### Optimization Strategies

#### Storage Backend
- **NodeRecord Store**: Use embedded key-value store (sled/rocksdb) for O(1) lookup
- **Frame Storage**: Use content-addressed filesystem storage for efficient retrieval
- **Head Index**: In-memory hash table for O(1) head resolution

#### Caching
- **NodeRecord Cache**: LRU cache for recently accessed nodes
- **Frame Content Cache**: Optional cache for frequently read frames
- **Hash Computation**: Cache intermediate hashes during tree construction

### Scalability Considerations
- **Large Filesystems**: System handles millions of files (O(1) node lookup)
- **Many Frames**: System handles large frame sets (bounded views keep queries fast)
- **Deep Trees**: System handles deep directory structures (O(d) updates)
- **Storage Growth**: Append-only storage grows linearly (can archive old data)

For detailed performance analysis, see **[Merkle Implementation](merkle_implementation.md)** and **[Implementation Guide](phase1_implementation.md)**.

---

## Constraints & Non-Goals

### Constraints

#### Determinism Requirement
- All operations must be deterministic
- No random number generation in core paths
- No time-dependent behavior (except metadata timestamps)
- No external API calls that could vary

#### No Search Constraint
- No semantic search or fuzzy matching
- No full scans of frame storage
- No content-based queries (only hash-based)
- No machine learning or AI in core engine

#### Append-Only Constraint
- Frames are immutable once created
- Nodes are immutable (new state = new NodeID)
- No deletion or modification of existing data
- History is preserved (can archive old data)

#### Bounded Context Constraint
- Context views have maximum frame count
- No unbounded frame retrieval
- Memory usage is bounded per operation

### Non-Goals (Out of Scope for Phase 1)

#### Not Included
- **Agent Workflows**: No agent read/write APIs (Phase 2)
- **Context Synthesis**: No branch/directory synthesis (Phase 2)
- **Regeneration**: No incremental regeneration (Phase 2)
- **Semantic Search**: No content-based search
- **Frame Deletion**: No deletion of frames (append-only)
- **Frame Modification**: No mutation of existing frames
- **Global Queries**: No queries across entire workspace
- **Distributed Storage**: No multi-machine storage
- **Frame Encryption**: No encryption of frame content
- **Access Control**: No access control (Phase 2 adds basic agent roles)

#### Future Phases
- **Phase 2**: Agent workflows, synthesis, regeneration
- **Phase 3+**: Advanced features (search, distributed storage, etc.)

---

## Quick Links

- **[Architecture Overview](phase1_architecture.md)** - System architecture and component relationships
- **[Component Specifications](phase1_components.md)** - Detailed specifications for each component
- **[Merkle Implementation](merkle_implementation.md)** - Merkle tree design decisions and implementation specification
- **[Development Phases](phase1_phases.md)** - Task breakdown and exit criteria
- **[Implementation Guide](phase1_implementation.md)** - Rust-specific implementation details

---

## Phase Exit Criteria

Phase 1 is complete when:
- **Deterministic ingestion**: Same filesystem → same root hash
- **Stable NodeID / FrameID**: Same content → same IDs
- **Zero-scan context retrieval**: O(1) or O(log n) access, no full scans
- **Hash-based invalidation**: Changes detected only through hash comparison
- **Bounded context views**: Context retrieval is bounded and deterministic
- **All components operational**: All six components implemented and tested
