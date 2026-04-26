# meld-world-model

The `meld-world-model` crate is the durable, queryable record of what has happened during execution. It maintains two independent materialized views over the event spine — **claims** and **anchors** — each persisted to an embedded [sled](https://github.com/spacejam/sled) database and reconstructable by replaying events.

---

## Concepts

### Claims

A claim is a settled fact about a domain object. When execution completes, fails, or produces output, the world model records a `ClaimRecord` asserting that outcome for a specific subject.

**`ClaimKind`** — the three settled states:
- `GenerationSucceeded` — a generation task completed successfully
- `GenerationFailed` — a generation task failed
- `ArtifactAvailable` — a task run produced a named artifact

Claims have a `SettlementStatus`: `Active` or `Superseded`. A new claim over the same subject supersedes the prior one; the full replacement chain is preserved.

Each claim carries `EvidenceRecord`s linking it to the source events and domain objects that caused it. `ProvenanceRecord` aggregates that evidence for reconstruction.

### Anchors

An anchor is a directed, perspective-scoped pointer: a subject pointing at a target, labeled by a `PerspectiveKey`. Anchors form a traversable knowledge graph over domain objects.

**`AnchorSelectionRecord`** — one resolved pointer:

| Field | Meaning |
|---|---|
| `subject` | the domain object that holds the pointer |
| `perspective` | a `(kind, id)` namespace classifying the relationship |
| `target` | the domain object being pointed at |

Three canonical perspective kinds are used by the system:

| `perspective_kind` | `perspective_id` | Meaning |
|---|---|---|
| `frame_type` | e.g. `"analysis"` | Current frame head for a node, by type |
| `snapshot` | `"current"` | Current snapshot for a source |
| `artifact_type` | e.g. `"summary"` | Current artifact for a task run, by type |

Anchors supersede each other per `(subject, perspective)`. History is preserved; only the latest is `Active`.

`TraversalFactRecord` is the raw event that drove an anchor change. `AnchorProvenanceRecord` links an anchor back to the spine facts and domain objects that created it.

---

## Storage

Both models use separate sled trees with structured secondary indexes:

| Store | Indexes maintained |
|---|---|
| `WorldStateStore` (claims) | by subject (active), by subject (history), by source fact, by sequence |
| `TraversalStore` (anchors) | by anchor ref (current/history), by subject+perspective, outgoing/incoming relations per object |

Both stores are append-oriented: records are written once and supersession updates a status field rather than deleting.

---

## Query API

### `WorldStateQuery`

Borrowed view over `WorldStateStore`.

```rust
query.current_claims_for_object(subject)         // active claims
query.claim_history_for_object(subject)          // all claims, including superseded
query.provenance_for_claim(claim_id)             // evidence trace
query.supersession_chain_for_claim(claim_id)     // replacement chain
```

### `TraversalQuery`

Borrowed view over `TraversalStore`.

```rust
query.current_anchor(anchor_ref)                           // anchor by ref
query.current_anchors_for_subject(subject)                 // all active anchors for a subject
query.anchor_history(anchor_ref)                           // full history for an anchor ref
query.current_frame_head(node, frame_type)                 // frame_type anchor
query.current_frame_heads_for_node(node)                   // all frame_type anchors for a node
query.current_snapshot_for_source(source)                  // snapshot anchor
query.current_artifact_for_task_run(task_run, type_id)     // artifact anchor
query.neighbors(object, direction, relation_types, current_only)  // adjacent objects
query.walk(start, spec)                                    // depth-limited graph traversal
query.provenance_for_anchor(anchor_id)                     // source facts for an anchor
```

### `WorldModelQueries`

`Arc`-wrapped, clone-safe handle exposing both stores. Passed to long-lived runtime components and the API layer.

---

## Reduction

Events from the spine are processed by two reducers:

**`WorldStateReducer`** — handles `execution.control.*` and `execution.task.*` events. Derives claim intents and writes `ClaimRecord`s to `WorldStateStore`. Emits `world_state.claim_added`, `world_state.claim_superseded`, and `world_state.evidence_attached` events.

**`TraversalReducer`** — handles domain events that imply structural relationships. Derives `TraversalIntent`s (`SelectAnchor` / `EndAnchor`) and writes anchor records to `TraversalStore`. Emits `world_state.anchor_selected` and `world_state.anchor_superseded` events.

**`GraphRuntime`** — drives the traversal side. Calls `catch_up()` to replay any unprocessed spine events through `TraversalReducer` on startup or re-initialization.

---

## Position in the system

`meld-world-model` sits downstream of the event spine and upstream of anything that needs to reason about current or historical execution state.

```
meld-events (spine)
    └── meld-world-model
            ├── WorldStateReducer  → claims store
            ├── TraversalReducer   → anchor/graph store
            └── WorldModelQueries  → consumed by:
                    ├── context reducer
                    ├── workspace reducer
                    ├── task reducer
                    ├── execution ports
                    ├── branches / CLI
                    └── API layer
```

It depends only on `meld-events` for event types, `DomainObjectRef`, and storage primitives.
