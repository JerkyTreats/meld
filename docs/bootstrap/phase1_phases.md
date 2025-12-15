# Phase 1 Development Phases

## Development Phases

### Phase 1A — Deterministic Identity

| Task | Status |
|-----|--------|
| Path canonicalization | Todo |
| Hash selection | Todo |
| NodeID generation | Todo |
| FrameID generation | Todo |

### Phase 1B — Filesystem Merkle Tree

| Task | Status |
|-----|--------|
| Filesystem walker | Todo |
| File hashing | Todo |
| Directory hashing | Todo |
| Root computation | Todo |

### Phase 1C — NodeRecord Store

| Task | Status |
|-----|--------|
| Schema definition | Todo |
| Persistence layer | Todo |
| Fast lookup API | Todo |

### Phase 1D — Context Frames

| Task | Status |
|-----|--------|
| Frame schema | Todo |
| Append workflow | Todo |
| Blob storage | Todo |

### Phase 1E — Frame Sets & Heads

| Task | Status |
|-----|--------|
| Frame membership tracking | Todo |
| Merkle set computation | Todo |
| Head update logic | Todo |

### Phase 1F — Context Views

| Task | Status |
|-----|--------|
| View schema | Todo |
| Selection algorithm | Todo |
| Determinism tests | Todo |

---

## Phase Exit Criteria

- Deterministic ingestion: Same filesystem → same root hash
- Stable NodeID / FrameID: Same content → same IDs
- Zero-scan context retrieval: O(1) or O(log n) access, no full scans
- Hash-based invalidation: Changes detected only through hash comparison

---

[← Back to Spec](phase1_spec.md) | [Next: Implementation →](phase1_implementation.md)

