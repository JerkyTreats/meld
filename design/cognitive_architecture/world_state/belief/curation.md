# Curation In Belief

Date: 2026-04-12
Status: active
Scope: natural runtime model for belief maintenance inside `world_state/belief`

## Thesis

To each domain be true.

The natural substrate for `belief` is a living world-model engine that consumes promoted facts and materializes current belief.
It is not primarily a workflow engine.

`curation` integrates.
It does not observe raw signal and it does not perform deliberate world side effects.

The substrate must support:

- sparse attached state
- many overlapping reducer passes
- belief revision over time
- provenance and supersession
- projection-specific materialized views

## Natural Runtime Shape

The substrate should look like:

- fact ingestors from the spine
- belief update systems
- supersession systems
- calibration systems
- projection builders for planner and operator views

This is why ECS is a plausible fit here.
It gives `belief` a way to maintain identity and many sparse attached belief concerns without forcing one rigid record shape for every internal operation.

## Core Primitives

- anchor
  cross-domain durable reference such as `DomainObjectRef`
- entity
  live world-state identity used to accumulate attached state
- evidence component family
  support, contradiction, source, and reliability attachments
- belief component family
  current confidence, validity, and revision state
- provenance component family
  why, when, and from which facts a belief changed
- projection
  materialized graph or query view derived from current belief state

## Materialization Posture

The knowledge graph is a materialized belief view over durable facts.

That implies:

- the spine is the replayable source history
- `curation` materializes current belief from that history
- planner and operator reads should target shaped materialized views
- dematerialization must remain possible through replay

The materialized belief view may be canonical as the current world model without becoming the irreducible source of truth.

## What The Substrate Must Support

- identity continuity
- evidence fusion
- contradiction handling
- confidence decay and revision
- supersession without provenance loss
- calibration from later outcomes
- multiple projections over the same underlying belief state

## What Should Not Be Forced Into This Substrate

- raw sensory transport
- task orchestration
- provider calls
- direct human-facing authoring workflows
- global sequencing policy

Those remain outside `curation`.

## Relationship To Spine

`belief` consumes the spine and materializes current belief from it.
It may also publish curated fact updates, but the spine remains the cross-domain durable ledger.

The ordering and temporal rules are not solved by the substrate alone.
They are inherited from spine semantics.

## Relationship To ECS

ECS is not the doctrine of `belief`.
It is a candidate internal substrate.

The useful boundary is:

- ECS may own live mutable belief internals
- graph-shaped records and shaped query views remain the public contract

That boundary keeps the world model living without turning every other domain into ECS by accident.

## First Slice

- thesis, evidence, provenance, supersession, and calibration records
- one curation replay path from promoted spine facts into current belief
- one planner-facing current belief projection
- one operator-facing inspection projection

## Read With

- [World State Domain](../README.md)
- [Belief](README.md)
- [Traversal](../traversal/README.md)
- [Knowledge Graph ECS Decision Memo](knowledge_graph_ecs_decision_memo.md)
- [Spine Concern](../../spine/README.md)
- [Observe Merge Push](../../observe_merge_push.md)
