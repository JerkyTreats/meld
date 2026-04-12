# Spine Concern

Date: 2026-04-12
Status: active
Scope: shared temporal substrate for sensory, world state, and execution

## Thesis

The event spine is the shared temporal substrate for the cognitive loop.
No domain concern should require direct process-to-process correctness calls.

The spine must provide:

- durable publication of domain facts
- ordered replay across concerns
- subscription for reducers and planners
- cross-domain anchors for objects, beliefs, and outcomes

## Boundary

`spine` owns:

- append semantics for durable domain facts
- sequencing, replay, and subscription behavior
- cross-domain envelope fields such as `domain_id`, `content_hash`, and `DomainObjectRef`
- genesis and catch-up behavior when a new domain attaches

`sensory`, `world_state`, and `execution` own event meaning.
`telemetry` consumes the spine downstream and does not own correctness.

## Current Anchors

- the control event spine is already the durable system center
- the multi-domain envelope already anticipates `knowledge_graph`
- the current sequencing model still assumes a small set of deliberate writers

## Unresolved Decision

- one coordinator process with a global sequencer
- per-process logical clocks with a merge layer
- a distributed log with leader election

This decision blocks genuine multi-process sensory and world-state workers.

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Sensory Domain](../sensory/README.md)
- [World State Domain](../world_state/README.md)
- [Execution Domain](../execution/README.md)
- [Events Design](../events/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
