# Frame Type Removal Spec

Date: 2026-02-21

## Objective

Remove the `frame_type` concept from the context model so that context per node is keyed only by agent. This spec is for future implementation; it is not part of the current refactor plan.

## Rationale

### Advantages of frame_type today

- Multiple frames per node per agent: the head index key `(NodeID, frame_type)` allows several logical "streams" per node (e.g. summary, analysis) with one head per stream.

### Disadvantages

- Additional layer of complexity: callers must understand and pass frame_type; head index, Frame, queue, and CLI all carry it.
- No external configuration: frame_type is only set via CLI flags or code defaults (e.g. `"context-{agent_id}"`, `"context-writer"`). There is no config file or agent-level configuration.
- Orphan risk: a one-off typo or ad-hoc string creates a new head stream that is never used again; context is effectively orphaned with no way to discover or clean it via configuration.

### Decision

Treat frame_type as unnecessary. The "stream" of context for a node should be identified by agent only: one head per (node, agent). If multiple logical streams per agent are needed later, they can be owned by the agent domain (e.g. agent config or convention) rather than a free-form string at call sites.

## Scope

- Head index: change key from `(NodeID, frame_type)` to `(NodeID, agent_id)`.
- Frame model: remove or fix frame_type (see Target state).
- FrameID computation: stop including frame_type in the hash, or use a single constant for new frames.
- API: get_head, put_frame, update_head, and all callers that take or return frame_type.
- Queue: generation requests no longer take frame_type; head is resolved by (node_id, agent_id).
- CLI: remove `--frame-type` from context generate, context get, and any other commands that expose it.
- Composition and views: remove frame_type-based filtering and ordering, or replace with agent-based semantics where needed.
- Persistence: head index file format and any frame storage that stores frame_type.

## Out Of Scope

- Changing the refactor phases (Phase 7, 8, 9, 10) or the current migration plan.
- Redesigning agent config to add "owned" frame types; this spec removes frame_type, it does not move it into agent config.

## Current State

| Component | Use of frame_type |
|-----------|-------------------|
| Head index | Key is `(NodeID, String)` where the string is frame_type. One head entry per (node, frame_type). |
| Frame | Struct field `frame_type: String`. Included in FrameID hash. |
| API | `get_head(node_id, frame_type)`, `put_frame` updates head for `frame.frame_type`, `ensure_agent_frame(frame_type: Option<String>)` defaults to `"context-{agent_id}"`. |
| Queue | `enqueue` / `enqueue_and_wait` take `frame_type: Option<String>`; default `"context-{agent_id}"`. Request identity includes frame_type for dedupe. |
| Generation | Plan and executor use literal `"context-writer"` for generated frames. |
| CLI | Context generate: `--frame-type`. Context get: `--frame-type` (filter). |
| Composition | `FrameFilter::ByType(filter_type)`, `OrderingPolicy::Type`; match on `frame.frame_type`. |
| Workspace status | `count_nodes_for_frame_type`; status uses `"context-{agent_id}"` per agent. |

## Target State

### Head index

- Key: `(NodeID, agent_id: String)`.
- Value: one head FrameID per (node, agent).
- Methods: `get_head(node_id, agent_id)`, `update_head(node_id, agent_id, frame_id)`. Remove any `frame_type`-based overloads. `get_all_heads_for_node(node_id)` returns all head frame IDs for that node (across agents).

### Frame

- Option A: Remove `frame_type` field from the Frame struct. FrameID computation becomes `hash(basis || agent_id || content)` with no type component. Existing stored frames that contain frame_type in serialized form may require a migration or a compatibility read path.
- Option B: Keep a single constant frame_type (e.g. `"context"`) for all new frames for backward compatibility with existing storage; do not expose it at API or CLI. FrameID uses that constant so all new frames are consistent.

### API

- `get_head(&self, node_id, agent_id)`.
- `put_frame(node_id, frame, agent_id)`: frame no longer carries frame_type; head is updated for (node_id, agent_id). If Frame struct still has a field for compatibility, set it from a constant or omit from hash.
- `ensure_agent_frame(node_id, agent_id, ...)`: no frame_type parameter; head is uniquely (node_id, agent_id).
- Remove or refactor `content_by_type`, `latest_frame_of_type`, `filter_by_type` to agent-based equivalents if still needed.

### Queue and generation

- Generation request identity: `(node_id, agent_id)` only; remove frame_type from dedupe and from request payloads.
- Plan/executor: when creating or storing frames, use the single canonical "context" notion for the agent (no literal `"context-writer"` or multiple types).

### CLI

- Remove `--frame-type` from `context generate` and `context get`. Filtering by agent remains via `--agent` where applicable.

### Composition and views

- Remove `FrameFilter::ByType` and `OrderingPolicy::Type` or redefine in terms of agent_id (e.g. filter by agent, order by agent).
- Update call sites that pass frame_type into composition.

### Persistence

- Head index persistence format: change key serialization from `(NodeID, frame_type)` to `(NodeID, agent_id)`. Provide a one-time migration or version bump to convert existing head index files or to drop legacy keys.

## Migration Approach

1. **Characterization**: Add or run tests that capture current behavior for get_head, put_frame, generate, and CLI context commands so that post-removal behavior can be compared (e.g. one head per agent per node).
2. **Head index**: Introduce new key shape `(NodeID, agent_id)` and new accessors; keep old key shape and accessors behind a feature or version until migration is done. Migrate in-memory and on-disk head index to new key format; handle existing data that used frame_type (e.g. map legacy `(node, "context-{agent_id}")` to `(node, agent_id)`).
3. **Frame and FrameID**: Decide Option A vs B; update `compute_frame_id` and Frame struct; handle existing frames (read path compatibility or one-time migration).
4. **API and queue**: Switch to agent-only head resolution; remove frame_type parameters and defaults.
5. **CLI**: Remove `--frame-type`; update help and any scripts/docs that reference it.
6. **Composition and views**: Remove or refactor frame_type-based filtering/ordering.
7. **Cleanup**: Remove frame_type from telemetry events, workspace status, and any remaining references. Delete deprecated head index key format and compatibility code.

## Risks and Mitigation

- **Existing data**: Head index and stored frames may contain frame_type. Mitigation: explicit migration path or versioned format; document upgrade steps.
- **Breaking CLI/API**: Callers that pass `--frame-type` or use get_head(node, type) will break. Mitigation: release note and doc update; no long-term compatibility promise if project avoids backward compatibility per AGENTS.md.
- **Multiple streams per agent**: If a use case later requires multiple named streams per agent per node, the design would need to reintroduce a concept owned by the agent (e.g. agent config). Mitigation: spec is "remove frame_type"; future "agent-owned stream names" would be a separate spec.

## Exit Criteria

- Head index is keyed only by (NodeID, agent_id). No frame_type in head index key or value.
- Frame struct and FrameID computation have no frame_type or use a single constant not exposed to callers.
- API and queue accept no frame_type parameters; head resolution is by (node_id, agent_id) only.
- CLI has no `--frame-type` option on context commands.
- Composition and views do not filter or order by frame_type.
- Tests and characterization suites pass; documentation updated.

## References

- [Context Domain Structure](context_domain_structure.md)
- [Head index](src/heads.rs): current key `(NodeID, frame_type)`
- [Frame ID computation](src/context/frame/id.rs): frame_type in hash
- AGENTS.md: avoid backwards compatibility; domain-first structure

## Status

Not part of current refactor. Spec is for later implementation.
