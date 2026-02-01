# Phase 2 API Specifications

## Core API Surface

### GetNode
```rust
async fn get_node(
    node_id: NodeID,
    view: ContextView,
) -> Result<NodeContext, ApiError>;
```

**Parameters:**
- `node_id`: The NodeID to retrieve context for
- `view`: ContextView policy specifying frame selection and ordering

**Returns:**
- `NodeContext`: Node record plus selected frames
- `ApiError`: Error if node not found or invalid request

**Behavior:**
- Deterministic: Same inputs → same outputs
- Read-only: Never triggers writes or synthesis
- Bounded: Frame count limited by view policy

**Example:**
```rust
let view = ContextView {
    max_frames: 100,
    ordering: OrderingPolicy::Recency,
    filters: vec![FrameFilter::ByType("analysis".to_string())],
};

let context = get_node(node_id, view).await?;
// Returns up to 100 most recent "analysis" frames for the node
```

---

### PutFrame
```rust
async fn put_frame(
    node_id: NodeID,
    frame: Frame,
    agent_id: String,
) -> Result<FrameID, ApiError>;
```

**Parameters:**
- `node_id`: The NodeID to attach frame to
- `frame`: Frame content (basis, content, metadata)
- `agent_id`: Identity of agent creating the frame

**Returns:**
- `FrameID`: The generated FrameID for the new frame
- `ApiError`: Error if node not found, agent unauthorized, or invalid frame

**Behavior:**
- Append-only: Creates new frame, never mutates existing
- Atomic: Frame creation and head update are transactional
- Deterministic: Same inputs → same FrameID

**Example:**
```rust
let frame = Frame {
    basis: Basis::Node(node_id),
    content: b"Analysis: This file contains...".to_vec(),
    metadata: HashMap::new(),
};

let frame_id = put_frame(node_id, frame, "agent-123".to_string()).await?;
// Frame is appended to node's frame set, head is updated atomically
```

---

### SynthesizeBranch
```rust
async fn synthesize_branch(
    node_id: NodeID,
    frame_type: String,
    agent_id: String,
    policy: Option<SynthesisPolicy>,
) -> Result<FrameID, ApiError>;
```

**Parameters:**
- `node_id`: Directory NodeID to synthesize
- `frame_type`: Type identifier for the synthesized frame
- `agent_id`: Identity of synthesis agent
- `policy`: Optional synthesis policy (default: concatenation)

**Returns:**
- `FrameID`: The generated FrameID for the synthesized frame
- `ApiError`: Error if node not found, not a directory, or synthesis fails

**Behavior:**
- Explicit: Only called via API, never implicit
- Bottom-up: Requires child frames to exist
- Deterministic: Same child frames → same synthesized frame

**Example:**
```rust
let policy = SynthesisPolicy::Concatenation;
let frame_id = synthesize_branch(
    dir_node_id,
    "directory-summary".to_string(),
    "synthesis-agent".to_string(),
    Some(policy),
).await?;
// Synthesizes a frame from all child frames of the directory
```

---

### Regenerate
```rust
async fn regenerate(
    node_id: NodeID,
    recursive: bool,
) -> Result<RegenerationReport, ApiError>;
```

**Parameters:**
- `node_id`: NodeID to regenerate frames for
- `recursive`: Whether to regenerate descendant nodes

**Returns:**
- `RegenerationReport`: Summary of regenerated frames
- `ApiError`: Error if node not found or regeneration fails

**Behavior:**
- Incremental: Only regenerates frames with changed basis
- Idempotent: Re-running produces same result
- Atomic: Regeneration is transactional

**Example:**
```rust
let report = regenerate(node_id, false).await?;
// Regenerates only frames for this node whose basis has changed
// Returns: { regenerated_count: 3, frames: [frame_id1, frame_id2, frame_id3] }
```

---

## Response Types

### NodeContext
```rust
struct NodeContext {
    node_id: NodeID,
    node_record: NodeRecord,
    frames: Vec<Frame>,  // Selected by ContextView policy
    frame_count: usize,  // Total frames (may exceed view limit)
}
```

### RegenerationReport
```rust
struct RegenerationReport {
    node_id: NodeID,
    regenerated_count: usize,
    frame_ids: Vec<FrameID>,
    duration_ms: u64,
}
```

---

## Error Types

See [Error Handling](phase2_spec.md#error-handling) section in the main spec for detailed error types and handling.

---

[← Back to Phase 2 Spec](phase2_spec.md)

