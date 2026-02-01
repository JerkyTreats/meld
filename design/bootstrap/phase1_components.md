# Phase 1 Component Specifications

## Major Components

### Filesystem Merkle Tree

#### Purpose
Represents the entire workspace filesystem as a Merkle tree, where each node (file or directory) has a deterministic hash based on its content and structure.

#### Requirements
- **Canonical path normalization**: All paths must be normalized (e.g., resolve `..`, `.`, remove trailing slashes, handle symlinks)
- **Deterministic child ordering**: Children must be ordered consistently (e.g., lexicographically by name)
- **Stable NodeID generation**: NodeID = hash(node content + children hashes + metadata)
- **Hash propagation on change**: When a node changes, all ancestor hashes must be recomputed

#### Data Structures
- **File Node**: `hash(file_content + metadata)` → NodeID
- **Directory Node**: `hash(sorted_children_hashes + metadata)` → NodeID
- **Root Hash**: Hash of the workspace root directory node

#### Test Criteria
- Identical trees produce identical roots
- Single file change invalidates ancestors only (not siblings or descendants)
- Re-ingestion is idempotent (same filesystem state → same root hash)

---

### NodeRecord Store

#### Purpose
Provides fast lookup of node metadata and relationships without requiring tree traversal. Acts as an index into the filesystem Merkle tree.

#### Requirements
- **O(1) lookup by NodeID**: Hash table or similar structure for constant-time access
- **Stores structural data + frame pointers**: Node metadata, children list, parent reference, and pointers to associated frame sets
- **No embedded frame content**: Frames are stored separately; only references are kept here

#### Data Structures
- **NodeRecord**: `{ NodeID, path, type (file/dir), children[], parent, frame_set_root, metadata }`
- **Storage**: Key-value store keyed by NodeID

#### Test Criteria
- Constant-time reads by NodeID
- Correct child enumeration matches filesystem structure
- Frame roots match recomputation from frame set

---

### Context Frames

#### Purpose
Immutable containers for context information (e.g., agent interactions, file analysis, metadata) associated with filesystem nodes. Each frame is content-addressed and append-only.

#### Requirements
- **Append-only**: Once created, frames cannot be modified (new context creates new frames)
- **Deterministic FrameID hashing**: `FrameID = hash(frame_content + basis)` where basis is the NodeID or parent FrameID
- **Explicit basis declaration**: Each frame must declare what it's based on (NodeID, previous FrameID, or both)
- **Truth-only hashing**: Only content that affects truth value is hashed (exclude timestamps, non-deterministic metadata)

#### Data Structures
- **Frame**: `{ FrameID, basis (NodeID/FrameID), content (blob), metadata (non-hashed), timestamp }`
- **FrameID**: `hash(basis + content)`

#### Test Criteria
- Same inputs → same FrameID (deterministic)
- Metadata does not affect hashes (only content + basis)
- Frames immutable after creation (no updates allowed)

---

### Context Frame Merkle Set

#### Purpose
Maintains a deterministic set of frames associated with a node. Uses a Merkle set structure to enable efficient membership verification and set comparison.

#### Requirements
- **Deterministic ordering**: Frames must be ordered consistently (e.g., by FrameID lexicographically)
- **Stable empty-set root**: Empty set has a well-defined root hash (e.g., hash of empty string or zero)
- **Root changes only on membership change**: Adding/removing frames changes root; reordering identical frames does not

#### Data Structures
- **Merkle Set**: Tree structure where leaves are FrameIDs, internal nodes are hashes of children
- **Set Root**: Root hash of the Merkle set tree
- **Ordering**: Deterministic sort by FrameID (or other canonical ordering)

#### Test Criteria
- Adding frame changes root deterministically (same frames added in any order → same root)
- Rebuild matches stored root (recomputation from stored frames produces identical root)
- Empty set has stable root hash

---

### Frame Heads

#### Purpose
Provides O(1) access to the "latest" or "current" frame for a given node, filtered by type (e.g., agent type, context type). Enables fast access without scanning frame sets.

#### Requirements
- **O(1) resolution by type / agent**: Hash table keyed by `(NodeID, type)` → FrameID
- **Pointer updates only**: Only the pointer is updated when new frames are added; frames themselves are immutable
- **No frame mutation**: Frames are never modified; new frames create new head pointers

#### Data Structures
- **Head Index**: `{ (NodeID, type) → FrameID }`
- **Update Logic**: When new frame added, if it matches type filter, update head pointer

#### Test Criteria
- No scans required (direct hash lookup)
- Multiple heads coexist safely (different types can have different heads for same node)
- Head updates are atomic and consistent

---

### Context Views

#### Purpose
Selects and orders a bounded set of frames for retrieval, based on policies (e.g., recency, relevance, type). Ensures deterministic, bounded context retrieval.

#### Requirements
- **Deterministic selection**: Same inputs and policy → same frame selection
- **Bounded frame count**: Maximum number of frames in view (e.g., 100, configurable)
- **Policy-driven ordering**: Selection and ordering based on explicit policies (e.g., most recent, by type, by agent)

#### Data Structures
- **View Policy**: `{ max_frames, ordering (recency/type/agent), filters (type/agent) }`
- **View Result**: `[FrameID₁, FrameID₂, ..., FrameIDₙ]` where n ≤ max_frames

#### Test Criteria
- Repeatable results (same node + policy → same view)
- Max frame limit enforced (never exceeds configured maximum)
- Policy ordering is deterministic

---

[← Back to Architecture](phase1_architecture.md) | [Next: Merkle Implementation →](merkle_implementation.md)

