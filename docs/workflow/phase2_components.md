# Phase 2 Component Specifications

## Major Components

### 1. Agent Read / Write Model

#### Description
Defines how agents interact with nodes and context frames. Establishes clear boundaries between read and write operations, ensuring agents can safely operate concurrently while maintaining data integrity.

#### Requirements
- **Clear separation of reader vs writer agents**: Agents are explicitly designated as readers or writers
- **Writers may only append frames, never mutate**: New context creates new frames; existing frames are immutable
- **Readers consume context via ContextView only**: Readers use policy-driven views, never direct frame access
- **Agent identity included in all writes**: Every frame includes `agent_id` in metadata and FrameID basis
- **Concurrent access safety**: Multiple agents can read/write simultaneously without corruption

#### Agent Roles
- **Reader Agents**: Can query context via `GetNode` API, cannot create frames
- **Writer Agents**: Can create frames via `PutFrame` API, can also read context
- **Synthesis Agents**: Special writer agents that generate branch/directory frames from child contexts

#### Data Structures
```rust
struct AgentIdentity {
    agent_id: String,
    role: AgentRole, // Reader | Writer | Synthesis
    capabilities: Vec<Capability>,
}

enum AgentRole {
    Reader,
    Writer,
    Synthesis,
}
```

#### Test Criteria
- Writer agents cannot overwrite frames (attempts return error)
- Reader agents never trigger writes (API enforces read-only)
- Agent attribution preserved in FrameIDs (agent_id in basis hash)
- Concurrent agents do not corrupt state (atomic operations, proper locking)
- Agent identity verified on all write operations

---

### 2. Context APIs (Core Workflows)

#### Description
Minimal, stateless API surface enabling agent interaction with the context engine. Provides deterministic read and write operations that preserve all Phase 1 invariants.

#### Requirements
- **GetNode(node_id, ContextView)**: Retrieve node context using policy-driven view
- **PutFrame(node_id, frame)**: Append new frame to node's frame set
- **Deterministic request → response mapping**: Same inputs always produce same outputs
- **Stateless API layer**: No server-side state between requests
- **Explicit synthesis triggers**: Synthesis only occurs via explicit API calls, never implicitly

#### API Signatures
```rust
// Read operation
async fn get_node(
    node_id: NodeID,
    view: ContextView,
) -> Result<NodeContext, ApiError>;

// Write operation
async fn put_frame(
    node_id: NodeID,
    frame: Frame,
    agent_id: String,
) -> Result<FrameID, ApiError>;

// Synthesis operation (explicit)
async fn synthesize_branch(
    node_id: NodeID,  // Directory node
    frame_type: String,
    agent_id: String,
) -> Result<FrameID, ApiError>;
```

#### NodeContext Response
```rust
struct NodeContext {
    node_id: NodeID,
    node_record: NodeRecord,
    frames: Vec<Frame>,  // Selected by ContextView policy
    frame_count: usize,  // Total frames (may exceed view limit)
}
```

#### Test Criteria
- Same request yields same frame set (deterministic selection)
- Frame append updates heads correctly (atomic head updates)
- API calls do not trigger synthesis implicitly (explicit only)
- Error handling is deterministic (same error for same conditions)
- Concurrent requests handled safely (no race conditions)
- Invalid node_id returns clear error (not panic)

---

### 3. Branch Context Synthesis

#### Description
Directory-level aggregation of child node context. Combines context frames from child nodes into a single synthesized frame for the parent directory. Synthesis is deterministic, bottom-up, and limited to explicit subtree scope.

#### Requirements
- **Bottom-up synthesis only**: Children must be synthesized before parents
- **Deterministic synthesis inputs**: Same child frames → same branch frame
- **Explicit frame_type per synthesis**: Each synthesis operation declares its frame type
- **No global context access**: Synthesis only uses declared child nodes
- **Basis construction**: Branch frame basis includes all child frame FrameIDs (ordered)

#### Synthesis Algorithm
1. Collect all child nodes of directory (from NodeRecord)
2. For each child, retrieve head frame of specified type (or all frames if type is "*")
3. Order child frames deterministically (by NodeID, then FrameID)
4. Construct basis: `hash(concat(sorted_child_frame_ids))`
5. Generate synthesis content: aggregate child frame contents (policy-driven)
6. Create new frame: `FrameID = hash(basis + content + frame_type)`
7. Append frame to directory's frame set
8. Update directory's head for the frame_type

#### Basis Construction
```rust
struct SynthesisBasis {
    node_id: NodeID,           // Directory node
    child_frame_ids: Vec<FrameID>,  // Ordered list
    frame_type: String,
    synthesis_policy: SynthesisPolicy,
}
```

#### Synthesis Policies
- **Concatenation**: Simple concatenation of child frame contents
- **Summarization**: Generate summary from child frames (deterministic algorithm)
- **Filtering**: Select subset of child frames based on criteria
- **Custom**: Pluggable synthesis function (must be deterministic)

#### Test Criteria
- Same child frames → same branch frame (deterministic)
- Child change invalidates parent basis (basis hash changes)
- Synthesis limited to subtree scope (no parent/sibling access)
- No synthesis without declared trigger (explicit API call required)
- Empty directory produces stable empty frame
- Synthesis preserves frame ordering (deterministic child ordering)

---

### 4. Incremental Regeneration

#### Description
Rebuilds derived context frames when their basis changes. Regeneration is incremental, localized, and basis-driven—only frames whose basis has changed are regenerated. Old frames are retained (append-only), ensuring full history preservation.

#### Requirements
- **Basis hash comparison for invalidation**: Compare stored basis hash with current basis hash
- **Regeneration produces new frames**: New frames with new FrameIDs (never mutate existing)
- **Old frames retained (append-only)**: All historical frames preserved
- **Heads updated atomically**: Head updates are transactional
- **Minimal scope**: Only regenerate frames whose basis changed
- **No cascading beyond declared scope**: Regeneration stops at declared boundaries

#### Regeneration Workflow
1. **Change Detection**: File change → NodeID change → basis hash change
2. **Invalidation**: Find all frames with changed basis (via basis index)
3. **Regeneration**: For each invalidated frame:
   - Retrieve current basis (child frames, node content, etc.)
   - Recompute frame content using same synthesis algorithm
   - Generate new FrameID (basis + content)
   - Append new frame (old frame remains)
4. **Head Update**: Update head pointers atomically for affected frame types
5. **Propagation**: If parent frames depend on this frame, mark for regeneration (limited scope)

#### Basis Index
```rust
// Index: basis_hash → Vec<FrameID>
// Enables fast lookup of frames affected by basis changes
type BasisIndex = HashMap<Hash, Vec<FrameID>>;
```

#### Regeneration Triggers
- **File content change**: NodeID changes → invalidates frames based on that node
- **Child frame change**: Child FrameID changes → invalidates parent synthesis frames
- **Explicit regeneration**: API call to regenerate specific frame or subtree

#### Test Criteria
- File change triggers minimal regeneration (only affected frames)
- Unchanged nodes are not touched (no unnecessary work)
- Regenerated FrameIDs differ predictably (deterministic but different)
- No cascading beyond declared scope (regeneration boundaries respected)
- Old frames remain accessible (history preserved)
- Regeneration is idempotent (re-running produces same result)
- Concurrent regeneration handled safely (no corruption)

---

### 5. Multi-Frame Composition

#### Description
Combining multiple context frames into composite views for agent consumption. Composition happens at read-time, is policy-driven, and produces bounded, deterministic results. No composite state is persisted—composition is computed on-demand.

#### Requirements
- **Composition is read-time only**: Computed when requested, not stored
- **No composite state persisted (yet)**: Future phase may add caching
- **Ordering is policy-driven**: Explicit policies determine frame selection and order
- **Bounded output size**: Maximum frame count enforced (from ContextView)
- **Deterministic**: Same inputs → same composition result

#### Composition Policies
- **Recency**: Most recent frames first (by timestamp)
- **Type Priority**: Order by frame type priority (configurable)
- **Agent Priority**: Order by agent priority (configurable)
- **Relevance Score**: Order by computed relevance (deterministic algorithm)
- **Custom**: Pluggable composition function

#### Composition Algorithm
1. Collect candidate frames from multiple sources:
   - Current node frames
   - Parent directory frames (if policy includes)
   - Sibling frames (if policy includes)
   - Related node frames (if policy includes)
2. Apply filters (by type, agent, date range, etc.)
3. Score and order frames (policy-driven)
4. Select top N frames (bounded by max_frames)
5. Return ordered list of FrameIDs (or full Frame objects)

#### Multi-Source Composition
```rust
struct CompositionPolicy {
    max_frames: usize,
    sources: Vec<CompositionSource>,  // Node, Parent, Siblings, Related
    ordering: OrderingPolicy,
    filters: Vec<FrameFilter>,
}

enum CompositionSource {
    CurrentNode,
    ParentDirectory,
    Siblings,
    RelatedNodes(Vec<NodeID>),
}
```

#### Test Criteria
- Composition never exceeds max frames (hard limit enforced)
- Ordering stable across runs (deterministic policy application)
- Missing frames handled gracefully (skip, don't fail)
- No writes during composition (read-only operation)
- Empty composition returns empty result (not error)
- Policy changes produce different results (non-deterministic across policies, deterministic within policy)

---

### 6. Tooling & Integration Layer

#### Description
Integration layer providing CLI tools, editor hooks, CI integration, and adapters for internal agents. Ensures the context engine can be used from various environments while maintaining determinism and idempotency.

#### Requirements
- **CLI or tool entrypoints**: Command-line interface for all operations
- **Workspace-scoped operations**: All operations scoped to workspace root
- **Idempotent tool execution**: Re-running tools produces same state
- **No UI-specific logic in core**: Core engine is UI-agnostic
- **Editor integration hooks**: File watchers, change notifications
- **CI integration**: Batch operations, validation, reporting

#### CLI Commands
```bash
# Context operations
merkle get-node <node_id> [--view <policy>]
merkle put-frame <node_id> <frame_file> [--agent <agent_id>]
merkle synthesize <node_id> [--type <frame_type>] [--agent <agent_id>]

# Regeneration
merkle regenerate <node_id> [--recursive]
merkle invalidate <node_id>

# Query operations
merkle list-frames <node_id> [--type <frame_type>]
merkle get-head <node_id> [--type <frame_type>]

# Workspace operations
merkle scan [--force]  # Rebuild tree from filesystem
merkle status          # Show workspace root hash, stats
merkle validate        # Verify integrity of all data structures
```

#### Editor Integration
- **File Watchers**: Monitor filesystem changes, trigger regeneration
- **Change Hooks**: Callbacks when nodes change (for editor updates)
- **LSP Integration**: Language server protocol support (future)

#### CI Integration
- **Batch Operations**: Process multiple nodes efficiently
- **Validation**: Verify workspace integrity
- **Reporting**: Generate reports on context state
- **Diff Generation**: Show context changes between runs

#### Agent Adapters
```rust
// Adapter for internal agents
trait AgentAdapter {
    fn read_context(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext>;
    fn write_context(&self, node_id: NodeID, frame: Frame) -> Result<FrameID>;
    fn synthesize(&self, node_id: NodeID, frame_type: String) -> Result<FrameID>;
}
```

#### Test Criteria
- Tools reproducible from CLI (same command → same result)
- Re-running tools yields same state (idempotent)
- Tool failures do not corrupt storage (atomic operations, rollback)
- Clear separation from core engine (core has no CLI dependencies)
- Workspace operations are isolated (no cross-workspace contamination)
- Error messages are clear and actionable

---

[← Back to Phase 2 Spec](phase2_spec.md)

