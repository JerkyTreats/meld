# Graph Requirements

Date: 2026-05-15
Status: active
Scope: expanded implementation requirements for `world_model/graph`

## Thesis

Graph implementation must prove a replayable path from spine facts to graph-readable traversal facts, current anchors, lineage, provenance, relation indexes, object history, branch presence, bounded traversal, and idempotent derived graph facts.

Graph is the shared structural substrate for upper world-model layers.
It does not settle belief, infer causality, detect regimes, or choose actions.

## Functional Requirements

### Source Facts

- Consume ordered spine facts after the last reduced sequence.
- Admit graph-readable events with object refs or relation refs.
- Preserve source spine fact id, sequence, event type, domain id, stream id, object refs, and relation refs.
- Record one traversal fact per admitted source event.
- Index traversal facts by source fact id and sequence.

### Object Identity

- Use `DomainObjectRef` as the public graph object identity.
- Create object projection state for every object mentioned by an admitted fact.
- Index fact membership by object.
- Support facts mentioning an object after a source sequence.
- Preserve object first seen and last seen cursors.

### Anchor Selection

- Represent current state as an anchor selected for a subject and perspective.
- Use `PerspectiveKey` with kind and id.
- Support current anchor by anchor ref.
- Support current anchor by subject and perspective.
- Support all current anchors for a subject.
- Support perspective-specific current anchor reads.
- End current anchor on tombstone or supersession.
- Preserve older anchors as history.

### Lineage

- Record supersession when a new anchor replaces a current anchor.
- Preserve prior anchor id, superseding anchor id, supersession sequence, source fact id, and derived fact id.
- Support anchor history by anchor ref.
- Support lineage chain by anchor id.

### Provenance

- Preserve source fact ids for anchors.
- Preserve derived fact ids for graph-owned publications.
- Preserve object refs and relation refs used by graph state.
- Support provenance query by anchor id.
- Keep provenance as explanation, not belief authority.

### Relations

- Index every event relation as outgoing and incoming.
- Preserve relation type, source object, target object, source fact id, and sequence.
- Support neighbor reads by object, direction, relation filter, and current-only flag.
- Support bounded walks with depth, direction, relation filters, current-only flag, and optional fact inclusion.

### Branch Scope

- Preserve branch-local object presence.
- Preserve branch-local anchor state where branch facts are local.
- Support branch-annotated federated traversal.
- Keep branch facts from merging into global authority without explicit federation policy.

### Derived Graph Facts

- Publish anchor selected events through the spine.
- Publish anchor superseded events through the spine.
- Use deterministic record ids for idempotent append.
- Include graph object refs in derived envelopes.
- Treat duplicate derived append as success.

### Runtime

- Maintain a durable reducer cursor.
- Hold one graph catch-up guard per runtime.
- Flush graph store and spine before advancing reducer cursor.
- Recover by replaying after durable cursor.
- Rebuild graph projections from spine history.

## Public Interface Requirements

- Query current anchor by anchor ref.
- Query current anchor by subject and perspective.
- Query all current anchors for a subject.
- Query current workspace snapshot for a source.
- Query current frame head for a node and frame type.
- Query current artifact for a task run and artifact type.
- Query anchor history by anchor ref.
- Query provenance by anchor id.
- Query facts for object after sequence.
- Query neighbors by direction, relation filter, and current-only flag.
- Query bounded graph walk by spec.
- Query branch presence for an object.
- Query branch-annotated traversal.

## Boundary Requirements

- Graph consumes spine facts.
- Graph publishes graph-derived facts through the spine.
- Graph exposes graph-shaped reads.
- Graph does not expose raw reducer internals as public contract.
- Graph does not assign trust or confidence.
- Graph does not classify contradiction.
- Graph does not infer causal effect.
- Graph does not decide regime identity.
- Graph does not choose planner or execution policy.

## Nonfunctional Requirements

### Replay

- Every graph projection is rebuildable from spine facts and reducer version.
- Every current anchor is rebuildable from anchor selection and end intents.
- Every lineage record is rebuildable from anchor supersession.
- Every relation index is rebuildable from event relations.

### Determinism

- Same source facts and reducer version produce same traversal facts, anchors, lineage, provenance, relation indexes, and derived fact ids.
- Source intent lowering is versioned.
- Derived graph event ids are deterministic.

### Idempotence

- Replaying the same spine facts does not duplicate traversal facts.
- Publishing the same derived graph fact uses the same record id.
- Graph catch-up can rerun after failure.

### Query Bounds

- Walk depth must be positive.
- Walks must respect relation filters.
- Walks must respect current-only selection.
- Query products must name their source cursor boundary when materialized.

### Audit

- Source fact ids survive fact lowering, object indexing, anchor selection, relation indexing, provenance, traversal, and derived publication.
- Superseded anchors remain inspectable.
- Current anchor changes are explainable through provenance and lineage.

## Read With

- [Graph Spec](spec.md)
- [Graph](README.md)
