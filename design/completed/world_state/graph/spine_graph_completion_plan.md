# Spine Graph Completion Review

Date: 2026-04-20
Status: implemented baseline
Scope: completion review for the first event spine plus graph traversal feature

## Intent

This document records the current post-implementation state of the event spine plus graph traversal substrate.

Earlier versions of this document described open branch work.
Most of that branch-era work has landed.
The remaining items are now limits of the baseline rather than blockers for the first graph slice.

## Implemented Goal

The application now has a real event spine plus graph traversal substrate.

Promoted cross-domain facts can be appended once, replayed deterministically, materialized into current anchors and relations, and queried within one branch or across many branches with branch provenance preserved.

## Done

- the spine has one canonical envelope with runtime-wide sequence
- legacy event reads remain compatible with graph fields
- session pruning preserves canonical spine history
- publisher domains emit canonical facts with explicit object refs and relations
- workspace scan and watch publish promoted structural `workspace_fs` facts
- `world_state/graph` rebuilds current anchors, lineage, provenance, relation adjacency, and bounded walks from replayable facts
- graph runtime appends durable derived anchor facts with idempotent record ids
- branch federation preserves branch presence, branch provenance, and branch scoped fact identity
- workflow task execution uses traversal to resolve final frame artifacts
- CLI startup and command completion catch graph up and record branch graph status

## Still Out Of Scope

- `world_state/belief`
- curation
- sensory raw lanes
- distributed sequencing
- generic graph query language
- non filesystem branch kinds as first class runtime implementations
- planner confidence policy

## Domain Matrix

| Domain | Current state | Notes |
| --- | --- | --- |
| `events` | implemented | canonical envelope, append, replay, idempotent record lookup, compatibility defaults |
| `session` | implemented split | session lifecycle no longer owns canonical history retention |
| `workspace` | implemented first slice | scan and watch publish source, snapshot, selected snapshot, and node facts |
| `context` | implemented publisher | frames and heads publish object refs and relations |
| `control` | implemented publisher | execution outcomes publish graph usable refs |
| `task` | implemented publisher and reducer input | task runs, artifacts, artifact slots, target links, and artifact selections feed traversal |
| `workflow` | implemented publisher and consumer | workflow facts are graph usable and task path resolution reads traversal |
| `world_state/graph` | implemented baseline | current anchors, lineage, provenance, facts, adjacency, walk, runtime catch up |
| `branches` | implemented baseline | branch annotated status, neighbors, walk, object presence, and federated fact ids |
| `cli` | implemented wiring | startup and command completion catch up graph and record branch status |
| `telemetry` | compatibility and reporting | downstream summaries do not own business fact meaning |

## Layer Matrix

| Layer | Current state | Notes |
| --- | --- | --- |
| spine substrate | implemented | append-only canonical history with runtime-wide sequence and legacy compatibility |
| publisher layer | implemented first slice | workspace, context, control, task, and workflow publish graph facts |
| graph materialization layer | implemented baseline | traversal facts, anchors, history, lineage, provenance, adjacency, bounded walk |
| federation layer | implemented baseline | branch annotated reads preserve presence and scoped fact identity |
| application consumer layer | implemented first path | workflow task path resolves final frame artifacts through traversal |
| belief layer | not implemented | explicitly deferred |

## Current Baseline Details

The event spine lives under `src/events`.
`src/telemetry` re-exports compatibility types and remains a downstream reporting layer.

The graph runtime lives under `src/world_state/graph`.
It reads canonical events, stores traversal materialization in sled trees, appends derived anchor events, and records the last reduced sequence.

Branch federation lives under `src/branches/query.rs`.
Federated reads return branch ids, canonical locators, object presence rows, federated fact ids, and skipped branch metadata.

Workflow consumes traversal in `src/workflow/executor.rs` when resolving final frame artifacts for task-backed workflow paths.

## Remaining Limits

The baseline should not be described as full cognitive belief.

Remaining work:

- define `world_state/belief` contracts
- design curation workers and settlement rules
- decide how legacy claim projection retires into belief
- add sensory promoted facts after graph semantics stay stable
- add richer planner-facing belief views
- add branch kinds beyond `workspace_fs` only after branch identity remains stable under real use

## Verification Matrix

| Area | Current proof |
| --- | --- |
| spine | runtime-wide monotonic sequence, legacy compatibility, append safety after session cleanup |
| workspace | scan publication, watch publication, stable source identity, snapshot selection correctness |
| graph materialization | replay parity, current anchor lookup, lineage, provenance, bounded walk |
| federation | one-branch parity, many-branch provenance preservation, branch scoped fact ids, unhealthy branch isolation |
| consumers | workflow task final frame resolution through traversal |

## Exit Review

The first spine plus graph traversal feature can be treated as landed when reviewing world state design.

Future design docs should not say that graph facts, event graph fields, derived anchor facts, workspace watch publication, branch annotated federation, or production traversal consumption are pending.

Future docs should instead treat belief, curation, sensory promotion, richer planner views, and non filesystem branch kinds as the next scope.

## Read With

- [Graph](../../../cognitive_architecture/world_state/graph/README.md)
- [Graph Implementation Status](implementation_plan.md)
- [Temporal Fact Graph](../../../cognitive_architecture/world_state/graph/temporal_fact_graph.md)
- [Workspace FS Graph Transition Status](workspace_fs_transition_requirements.md)
- [Branch Federation Substrate](../../../cognitive_architecture/world_state/graph/branch_federation_substrate.md)
- [Spine Concern](../../../cognitive_architecture/spine/README.md)
