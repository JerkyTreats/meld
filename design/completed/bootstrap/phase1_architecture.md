# Phase 1 Architecture

## Architecture Overview

The Phase 1 system consists of six core components that work together to provide a deterministic, hash-based filesystem state management system:

1. **Filesystem Merkle Tree**: Represents the directory structure and file contents as a Merkle tree
2. **NodeRecord Store**: Fast lookup storage for node metadata and relationships
3. **Context Frames**: Immutable, append-only context containers
4. **Context Frame Merkle Set**: Deterministic set of frames associated with nodes
5. **Frame Heads**: Efficient pointers to the latest frames by type/agent
6. **Context Views**: Policy-driven selection of frames for retrieval

All components rely on deterministic hashing and canonical ordering to ensure reproducibility and fast invalidation.

---

## Component Relationships

```
Filesystem Merkle Tree
    ↓ (generates)
NodeRecord Store
    ↓ (references)
Context Frame Merkle Set
    ↓ (contains)
Context Frames
    ↑ (indexed by)
Frame Heads
    ↓ (used by)
Context Views
```

**Data Flow:**
1. Filesystem changes → Merkle tree recomputation → NodeID changes
2. NodeID changes → NodeRecord updates → Frame set invalidation
3. New context → New Frame → Frame set update → Head update
4. Query → Context View → Policy selection → Frame retrieval

**Dependencies:**
- NodeRecord Store depends on Filesystem Merkle Tree (for NodeIDs)
- Context Frame Merkle Set depends on Context Frames (contains FrameIDs)
- Frame Heads depend on Context Frames (points to FrameIDs)
- Context Views depend on Frame Heads and Frame Sets (for selection)

---

## System Properties

### Determinism
- Same filesystem state → same root hash
- Same frame content → same FrameID
- Same frame set → same set root

### Performance
- O(1) node lookup by NodeID
- O(1) frame head resolution
- O(log n) frame set operations
- O(d) tree updates where d = depth

### Losslessness
- Full tree reconstruction from root + node store
- Full frame set reconstruction from root + frame store
- No information loss in hashing process

---

[← Back to Phase 1 Spec](phase1_spec.md) | [Next: Components →](phase1_components.md)

