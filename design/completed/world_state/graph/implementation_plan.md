# Graph Implementation Status

Date: 2026-04-20
Status: implemented baseline
Scope: status snapshot for the first canonical `world_state/graph` graph slice

## Summary

The first graph slice has landed.

The repo now has a materialized `world_state/graph` domain downstream of the shared event spine.

Implemented assumptions:

- the spine is the durable temporal source
- the graph is the current graph and traversal layer
- `DomainObjectRef` is the cross-domain anchor
- `EventRelation` is the cross-domain edge contract
- reducers own materialization
- traversal queries serve planner, workflow, operator, and branch-read surfaces without raw spine scans on common paths

The first version still does not attempt belief calculus, full sensory integration, distributed sequencing, or generic graph query language support.

## Implemented End State

The repo now has:

- one shared object reference contract in `src/events/contracts.rs`
- one canonical event envelope in `src/events.rs`
- runtime-wide append and replay in `src/events/store.rs`
- legacy event compatibility through serde defaults and telemetry re-exports
- one typed `world_state/graph` area with contracts, events, reducer, runtime, store, query, and compatibility adapter
- one replayable materialization path from spine facts into current graph state
- object history indexes
- relation adjacency indexes
- current anchor indexes
- anchor history and lineage indexes
- provenance lookup for anchors
- branch annotated federated reads over traversal stores

The graph can answer:

- what is the current anchor for object X and perspective Y
- why is anchor Z current
- what prior anchor did Z replace
- which source spine fact moved the anchor
- which objects neighbor object X
- which bounded subgraph is reachable from object X
- which current snapshot, frame head, or task artifact is selected

## Non Goals

- raw sensory lanes
- external graph database integration
- distributed log work
- arbitrary graph query language
- global ECS adoption
- belief confidence, contradiction, calibration, or Bayesian revision

## Implemented Contract

`DomainObjectRef` carries:

- `domain_id`
- `object_kind`
- `object_id`

`EventRelation` carries:

- `relation_type`
- `src`
- `dst`

Canonical event records carry:

- runtime-wide `seq`
- `record_id` for idempotent derived writes
- `domain_id`
- `stream_id`
- `event_type`
- `recorded_at`
- optional `occurred_at`
- optional `content_hash`
- explicit `objects`
- explicit `relations`
- JSON `data`

## Implemented Runtime

`GraphRuntime` owns catch up from the spine into traversal indexes.

Catch up:

- reads after `TraversalStore.last_reduced_seq`
- applies `TraversalReducer`
- appends emitted derived envelopes idempotently
- flushes the spine and traversal store
- records the last reduced sequence

Derived graph events currently include:

- `world_state.anchor_selected`
- `world_state.anchor_superseded`

The derived events use stable record ids so repeated catch up does not duplicate them.

## Implemented Indexes

Traversal storage currently maintains:

- `traversal_facts`
- `traversal_fact_objects`
- `traversal_object_facts`
- `traversal_outgoing_relations`
- `traversal_incoming_relations`
- `traversal_anchor_records`
- `traversal_current_anchor`
- `traversal_anchor_history`
- `traversal_anchor_lineage`
- `traversal_source_fact_index`
- `traversal_seq_index`
- `traversal_subject_perspective_index`
- `traversal_runtime_meta`

Legacy claim storage remains separate and maintains:

- world state facts
- claims
- evidence
- claim evidence links
- active claims by subject
- claim history by subject
- claim supersession
- source fact and sequence indexes

## Implemented Reducers

Traversal reducer source domains:

- `workspace_fs`
- `context`
- `execution`

Domain specific reducer intent providers live in:

- `src/workspace/reducer.rs`
- `src/context/reducer.rs`
- `src/task/reducer.rs`

They select anchors for:

- workspace snapshot heads
- context frame heads
- execution artifact slots

The graph also indexes relations from control, task, workflow, context, and workspace facts when those facts are present in the spine.

## Publisher Coverage

Graph-readable publishers now exist in:

- `src/workspace/events.rs`
- `src/context/events.rs`
- `src/control/events.rs`
- `src/task/events.rs`
- `src/workflow/events.rs`

Workspace scan and watch both publish canonical `workspace_fs` snapshot and node facts at promoted structural boundaries.

Context publishes frames and head selection.

Task publishes task run, artifact, artifact slot, target, and selection relations.

Workflow publishes workflow, thread, turn, plan, target, and produced-frame relations.

Control publishes plan, workspace target, and frame result relations.

## Query Surfaces

`TraversalQuery` exposes:

- current anchor by anchor ref
- current anchor by subject and perspective
- all current anchors for a subject
- anchor history
- neighbors
- bounded walk
- facts for object after sequence
- provenance for anchor
- current snapshot for workspace source
- current frame head for workspace node and frame type
- current artifact for task run and artifact type

`LegacyClaimAdapter` exposes current generation-style claims over traversal anchors for compatibility.

`BranchQueryRuntime` exposes branch annotated status, neighbors, and walk results.

## Operational Coupling

Graph catch up is wired into CLI startup and command completion through `RunContext`.

Branch runtime records graph catch up success and failure against branch metadata.

Workflow task execution now resolves final frame artifacts through traversal anchors.
That makes traversal part of the execution loop, not only an inspection surface.

## Verification

Implemented integration coverage includes:

- runtime-wide monotonic spine sequence
- legacy spine event compatibility with graph fields
- mixed old and new event replay with object refs
- session pruning preserving canonical spine history
- workspace scan publication
- workspace watch publication
- graph current anchor lookup
- graph replay parity for context heads
- graph current snapshot lookup
- graph task artifact selection
- graph walk across workspace, context, workflow, task, and plan objects
- durable derived anchor events after restart
- idempotent graph runtime catch up
- CLI scan bootstrapping graph runtime
- branch query parity with local traversal
- deterministic many-branch neighbor merge
- branch annotated object presence
- branch scoped federated fact identity
- unhealthy branch isolation

## Remaining Limits

- `world_state/belief` is not implemented
- curation is not implemented
- confidence, contradiction handling, calibration, and Bayesian revision remain out of scope
- sensory raw lanes are not graph publishers yet
- graph query is bounded traversal, not a generic query language
- legacy claim projection remains alongside traversal and should be retired only with compatibility tests

## Read With

- [World State Domain](../../../cognitive_architecture/world_state/README.md)
- [Temporal Fact Graph](../../../cognitive_architecture/world_state/graph/temporal_fact_graph.md)
- [Belief](../../../cognitive_architecture/world_state/belief/README.md)
- [Spine Concern](../../../cognitive_architecture/spine/README.md)
- [Multi-Domain Spine](../../../cognitive_architecture/events/multi_domain_spine.md)
