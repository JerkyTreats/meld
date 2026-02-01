# Phase 2 Architecture

## Component Relationships

### Data Flow
```
Agent Request
    ↓
Context APIs (GetNode/PutFrame)
    ↓
Agent Read/Write Model (Authorization)
    ↓
NodeRecord Store / Frame Storage
    ↓
Context Views / Frame Heads
    ↓
Multi-Frame Composition (if needed)
    ↓
Response to Agent
```

### Synthesis Flow
```
File Change
    ↓
NodeID Change
    ↓
Basis Hash Change
    ↓
Invalidation Detection
    ↓
Incremental Regeneration
    ↓
Branch Synthesis (if needed)
    ↓
New Frame Creation
    ↓
Head Update
```

### Regeneration Flow
```
Basis Change Detected
    ↓
Basis Index Lookup
    ↓
Find Affected Frames
    ↓
For Each Frame:
    Retrieve Current Basis
    Recompute Content
    Generate New FrameID
    Append New Frame
    ↓
Update Heads Atomically
    ↓
Propagate to Parents (if in scope)
```

### Composition Flow
```
Context View Request
    ↓
Collect Candidate Frames
    (Current Node, Parent, Siblings, Related)
    ↓
Apply Filters
    (Type, Agent, Date Range)
    ↓
Score and Order
    (Policy-Driven)
    ↓
Select Top N Frames
    (Bounded by max_frames)
    ↓
Return Ordered Frame List
```

---

## Dependencies

### Component Dependencies
- **Context APIs** depend on **Agent Read/Write Model** (for authorization)
- **Branch Synthesis** depends on **Context APIs** (to read child frames)
- **Incremental Regeneration** depends on **Branch Synthesis** (to regenerate synthesized frames)
- **Multi-Frame Composition** depends on **Context Views** (for frame selection)
- **Tooling & Integration** depends on all components (provides external interface)

### Phase 1 Dependencies
All Phase 2 components build upon Phase 1 components:
- **Agent Read/Write Model** uses Phase 1 **Context Frames** and **Frame Heads**
- **Context APIs** use Phase 1 **NodeRecord Store** and **Context Views**
- **Branch Synthesis** uses Phase 1 **Context Frame Merkle Set** and **Frame Heads**
- **Incremental Regeneration** uses Phase 1 **Filesystem Merkle Tree** for change detection
- **Multi-Frame Composition** uses Phase 1 **Context Views** for frame selection
- **Tooling & Integration** uses all Phase 1 components via APIs

---

## System Architecture

### Layer Structure
```
┌─────────────────────────────────────┐
│   Tooling & Integration Layer       │  (CLI, Editor, CI)
├─────────────────────────────────────┤
│   Context APIs                      │  (GetNode, PutFrame, Synthesize)
├─────────────────────────────────────┤
│   Agent Read/Write Model            │  (Authorization, Roles)
├─────────────────────────────────────┤
│   Workflow Components               │  (Synthesis, Regeneration, Composition)
├─────────────────────────────────────┤
│   Phase 1 Components                │  (Tree, Store, Frames, Views)
└─────────────────────────────────────┘
```

### Data Flow Between Layers
1. **External Request** → Tooling Layer (CLI/Editor/CI)
2. **API Call** → Context APIs Layer
3. **Authorization** → Agent Read/Write Model
4. **Data Access** → Phase 1 Components (NodeRecord Store, Frame Storage)
5. **Workflow Processing** → Workflow Components (if needed)
6. **Response** → Back through layers to external caller

---

## Concurrency Model

### Agent Concurrency
- Multiple agents can read simultaneously (no locking needed for reads)
- Multiple agents can write simultaneously (atomic frame append operations)
- Head updates are atomic (transactional)
- Basis index updates are atomic (transactional)

### Locking Strategy
- **Read Operations**: No locks required (immutable data)
- **Write Operations**: Per-node locks for frame appends
- **Head Updates**: Atomic transactions
- **Synthesis**: Per-directory locks (prevents concurrent synthesis of same directory)

---

## Storage Architecture

### Frame Storage
- Content-addressed storage: `frames/{FrameID[0..2]}/{FrameID[2..4]}/{FrameID}`
- Append-only: New frames never overwrite existing
- Immutable: Frames cannot be modified after creation

### Basis Index
- In-memory hash map: `basis_hash → Vec<FrameID>`
- Persisted periodically or on shutdown
- Enables fast invalidation lookup

### Head Index
- In-memory hash map: `(NodeID, FrameType) → FrameID`
- Updated atomically on frame append
- Persisted periodically or on shutdown

---

[← Back to Phase 2 Spec](phase2_spec.md) | [Next: Components →](phase2_components.md)

