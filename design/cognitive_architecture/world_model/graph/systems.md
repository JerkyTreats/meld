# Graph Systems

Date: 2026-05-15
Status: active
Scope: ECS systems for `world_model/graph`

## Thesis

Graph systems reduce spine facts into current anchors, lineage, provenance, relation indexes, object history, branch presence, traversal products, and idempotent derived graph facts.

They are deterministic over event order, reducer version, source intents, and storage state.
They do not infer belief, causal effect, regime identity, or action policy.

## Pipeline Overview

| Phase | System | Required input | Required output |
|---|---|---|---|
| 1 | graph catch-up | spine event store, reducer cursor | event batch |
| 2 | fact admission | event batch | graph-readable events |
| 3 | traversal fact recording | graph-readable events | `TraversalFact` |
| 4 | object indexing | traversal facts | `WorldObject`, object history index |
| 5 | relation indexing | traversal facts | `RelationEdge` |
| 6 | source intent lowering | graph-readable events | anchor selection or end intents |
| 7 | anchor selection | anchor intents | `Anchor` current state |
| 8 | lineage update | superseded anchors | `AnchorLineage` |
| 9 | provenance projection | facts, anchors, relations | `ProvenanceBundle` |
| 10 | branch federation | branch-aware facts | `BranchPresence` |
| 11 | traversal projection | query specs, indexes | `GraphWalk`, `NeighborSet` |
| 12 | derived graph publication | anchor and lineage changes | `DerivedGraphFact` |
| 13 | reducer cursor commit | successful reduction and publication | `GraphReductionCursor` |
| 14 | recovery and rebuild | spine facts and reducer cursor | rebuilt graph projections |

Replay invariant:
same spine events, reducer version, source intent rules, and starting cursor produce the same graph records, derived fact ids, and query results.

## Required Read Ports

| Port | Owner | Required for |
|---|---|---|
| spine replay | events | graph catch-up |
| spine idempotent append | events | derived graph publication |
| domain object refs | events and source domains | object indexing |
| event relations | events and source domains | relation indexing |
| branch refs | branch domain or graph facts | branch federation |
| traversal store | graph | current anchor and traversal reads |

## Graph Catch-Up

Input:
event spine and graph reduction cursor.

Output:
ordered event batch after last reduced sequence.

Rules:

- Read events after the last reduced sequence.
- Hold one catch-up guard per graph runtime.
- Preserve event sequence order.
- Advance reducer cursor only after graph records and derived publications are durable.

## Fact Admission

Input:
event batch.

Output:
graph-readable events.

Rules:

- Admit events from domains that publish graph-readable objects or relations.
- Preserve event type, domain id, stream id, source sequence, object refs, and relation refs.
- Reject events without graph-readable payload by returning no graph intents.
- Do not interpret trust, confidence, or action semantics.

## Traversal Fact Recording

Input:
graph-readable events.

Output:
`TraversalFact`.

Rules:

- Create one traversal fact per admitted source event.
- Record source spine fact id and source sequence.
- Record objects and relations exactly as published.
- Index source fact id to traversal fact id.
- Index sequence to traversal fact id.

## Object Indexing

Input:
traversal facts.

Output:
`WorldObject` records and object history indexes.

Rules:

- Create object identity for every object ref mentioned by a traversal fact.
- Index fact membership by object.
- Preserve facts mentioning object after sequence queries.
- Maintain first and last seen source cursor.

## Relation Indexing

Input:
traversal facts with relation refs.

Output:
`RelationEdge`.

Rules:

- Index outgoing relation by source object, relation type, target object, sequence, and fact id.
- Index incoming relation by target object, relation type, source object, sequence, and fact id.
- Preserve relation source fact and sequence.
- Do not derive causal relation from reachability.

## Source Intent Lowering

Input:
graph-readable event and source fact id.

Output:
anchor selection or anchor end intents.

Rules:

- Lower workspace snapshot selection into snapshot current anchor intent.
- Lower context head selection into frame head anchor intent.
- Lower context head tombstone into anchor end intent.
- Lower task artifact emission into artifact slot anchor intent.
- Return no intent when event data lacks required object refs.
- Keep source intent rules versioned.

## Anchor Selection

Input:
anchor selection and end intents.

Output:
`Anchor` records and current anchor index.

Rules:

- Select current target for one anchor ref, subject, and perspective.
- Use source sequence in anchor id.
- Ignore duplicate anchor selection when the same anchor is already current.
- Ignore older selection when current anchor has a later sequence.
- End current anchor before selecting a newer replacement.
- Clear current anchor when an end intent applies.

## Lineage Update

Input:
prior current anchor and new anchor.

Output:
`AnchorLineage`.

Rules:

- Record prior anchor id and superseding anchor id.
- Record supersession sequence.
- Preserve supersession derived fact id.
- Keep lineage append-only.

## Provenance Projection

Input:
traversal facts, anchors, relations, lineage, and derived graph facts.

Output:
`ProvenanceBundle`.

Rules:

- Attach source fact ids to anchor records.
- Attach derived fact ids to anchor selected and anchor superseded publication.
- Preserve object and relation refs that explain graph state.
- Provide provenance by anchor id.

## Branch Federation

Input:
branch-aware graph facts and federated traversal stores.

Output:
`BranchPresence`.

Rules:

- Preserve object presence per branch.
- Preserve branch-local anchor state.
- Preserve inherited presence separately from local presence.
- Keep branch-scoped reads from merging branch facts into one global authority.

## Traversal Projection

Input:
object indexes, relation indexes, current anchor state, and query specs.

Output:
`GraphWalk` and `NeighborSet`.

Rules:

- Support outgoing, incoming, and both-direction traversal.
- Support relation type filters.
- Require positive walk depth.
- Support current-only selection.
- Include facts only when requested.
- Preserve source cursor in query products.

## Derived Graph Publication

Input:
anchor selection and lineage changes.

Output:
`DerivedGraphFact` and spine append.

Rules:

- Publish anchor selected facts with idempotent record ids.
- Publish anchor superseded facts with idempotent record ids.
- Include graph object refs in derived envelopes.
- Append through the spine idempotent append contract.
- Treat duplicate derived publication as success.

## Reducer Cursor Commit

Input:
successful graph records and derived publication results.

Output:
updated `GraphReductionCursor`.

Rules:

- Track max source sequence observed.
- Include derived publication sequence when it exceeds source sequence.
- Flush spine and graph store before cursor update completes.
- Never advance cursor past unpersisted graph state.

## Recovery And Rebuild

Input:
spine event history and graph reduction cursor.

Output:
rebuilt graph projections.

Rules:

- Rebuild graph state from spine replay.
- Recreate traversal facts, object indexes, relation indexes, anchors, lineage, and provenance.
- Recreate derived publication ids deterministically.
- Preserve idempotence on derived fact append.
- Recover from reducer failure by replaying after the durable cursor.

## Failure Semantics

| Condition | Graph behavior |
|---|---|
| event lacks graph-readable refs | no graph intent |
| event relation has invalid object ref | route or reducer error |
| duplicate anchor selected fact | idempotent success |
| older anchor selection arrives after newer current anchor | ignore older selection |
| current anchor is tombstoned | clear current anchor |
| walk depth is zero | query validation error |
| derived fact append duplicates existing record | idempotent success |
| reducer fails before cursor commit | replay from previous cursor |

## Test Matrix

| Test | Expected proof |
|---|---|
| replay determinism | same spine facts produce same anchors and indexes |
| current anchor selection | latest valid anchor is current |
| older selection guard | older selection does not replace newer current anchor |
| anchor end | tombstone clears current anchor |
| lineage preservation | superseded anchor points to replacement |
| provenance preservation | anchor provenance names source facts |
| relation direction | incoming and outgoing neighbor reads differ correctly |
| bounded walk | max depth and relation filters are enforced |
| current-only walk | current-only traversal respects current state |
| idempotent publication | repeated catch-up does not duplicate derived graph facts |
| recovery | reducer rebuilds from prior durable cursor |
| branch read | branch-local presence remains branch scoped |

## System Rules

- Graph systems are deterministic over event order and reducer version.
- Graph systems mutate graph-owned projection records only.
- Graph systems publish derived graph facts through the spine.
- Graph systems preserve source fact refs and sequence boundaries.
- Graph systems expose traversal and current state without belief settlement.

## Read With

- [Graph Entities](entities.md)
- [Graph Components](components.md)
- [Graph Requirements](requirements.md)
- [Graph](README.md)
