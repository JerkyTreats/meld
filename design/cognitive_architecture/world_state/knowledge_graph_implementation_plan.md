# Knowledge Graph Implementation Plan

Date: 2026-04-12
Status: active
Scope: phased delivery plan for the first canonical `world_state` knowledge graph

## Summary

The first knowledge graph should land as a materialized `world_state` domain built downstream of the shared event spine.

This plan assumes:

- the spine remains the durable temporal source
- the graph is the current settled belief layer
- `DomainObjectRef` becomes the cross-domain anchor
- reducers own settlement
- planner reads projections, not the raw spine

The first version should not attempt full sensory integration, distributed sequencing, or generic graph query language support.

## Target End State

At the end of this plan, the repo should have:

- one shared object reference contract across domains
- one typed `world_state` domain with durable fact families
- one replayable settlement path from spine facts into current graph state
- one object history index
- one current claim index
- one planner-facing belief projection
- one operator-facing provenance projection

The first graph should answer:

- what do we currently believe about object X
- why do we believe claim Y
- what superseded claim Y
- which execution or workspace fact changed belief about object X

## Non Goals

- raw sensory lanes
- external graph database integration
- distributed log work
- arbitrary graph query language
- global ECS adoption

## Phase 0 Contract Freeze

Goal:

- freeze the minimum canonical model before code lands

Deliver:

- `DomainObjectRef`
- typed relation contract
- minimum durable fact shape
- first `world_state.*` fact family names
- first projection boundaries

Required decisions:

- `DomainObjectRef` field names and validation rules
- relation type naming rules
- which spine fields are required versus optional
- which domains are allowed to publish explicit object refs in the first slice

Acceptance gate:

- `Temporal Fact Graph` and `Multi-Domain Spine` agree on object reference and relation contracts
- no new world state doc still assumes filesystem identity is universal

## Phase 1 Spine Contract Lift

Goal:

- make the spine graph-capable without breaking legacy reads

Code landing zones:

- `src/telemetry/events.rs`
- `src/telemetry/sinks/store.rs`
- `src/telemetry/routing/ingestor.rs`
- `tests/integration/event_spine.rs`

Add:

- `DomainObjectRef`
- `EventRelation`
- optional `objects` field on canonical spine events
- optional `relations` field on canonical spine events
- explicit `recorded_at`
- explicit `occurred_at` where the source can provide it

Compatibility rule:

- all new fields must use serde defaults so old events remain readable

Acceptance gate:

- legacy execution events still read successfully
- new events can persist and replay with object refs and relations
- one integration test proves mixed old and new events replay cleanly

## Phase 2 World State Domain Skeleton

Goal:

- create the first real `world_state` runtime area

Code landing zones:

- `src/world_state.rs`
- `src/world_state/contracts.rs`
- `src/world_state/events.rs`
- `src/world_state/store.rs`
- `src/world_state/query.rs`

Define:

- typed record ids for claim, evidence, and settlement records
- `world_state.claim_added`
- `world_state.claim_revised`
- `world_state.claim_superseded`
- `world_state.evidence_attached`
- `world_state.calibration_recorded`

The first slice should keep public record shapes graph-oriented and typed.

Acceptance gate:

- world state record contracts round trip through serde
- world state event builders emit explicit object refs and relation lists
- no runtime reducer work yet

## Phase 3 Fact Store And Indexes

Goal:

- land the durable storage and traversal indexes needed for a real graph

Code landing zones:

- `src/world_state/store.rs`
- `src/world_state/query.rs`
- `tests/integration/world_state_graph.rs`

Required indexes:

- `fact_id -> fact`
- `object_ref -> fact ids`
- `claim_id -> evidence ids`
- `claim_id -> supersession chain`
- `object_ref -> current claim ids`
- `seq -> fact id`

Design rule:

- the spine stays the source of truth
- world state storage is an indexed materialization and query layer

Acceptance gate:

- facts can be loaded by object reference without scanning the whole spine
- current claims for one object can be loaded from indexes alone
- supersession chain lookup works for one claim lineage

## Phase 4 Settlement Reducers

Goal:

- convert spine facts into current graph state through deterministic reducers

Code landing zones:

- `src/world_state/reducer.rs`
- `src/world_state/projection.rs`
- `tests/integration/world_state_graph.rs`

First reducers:

- attachment reducer
  maps execution and workspace facts onto durable objects
- claim settlement reducer
  creates and revises current claims
- supersession and calibration reducer
  marks stale claims and records later outcome quality

First source domains:

- `execution`
- `workspace_fs`

`sensory` should wait until the reducer model is proven with slower and cleaner sources.

Acceptance gate:

- replay from spine facts rebuilds the same current claim view
- late arriving fact revises current claim without losing prior provenance
- one claim can attach to both a workspace node and a task run

## Phase 5 Planner And Operator Projections

Goal:

- make the graph useful to planning and inspection

Code landing zones:

- `src/world_state/projection.rs`
- `src/world_state/query.rs`
- future planner integration points under `src/control`

Planner-facing projection:

- current belief by object
- confidence
- staleness
- blocking unknowns

Operator-facing projection:

- claim provenance bundle
- supporting evidence
- contradicting evidence
- supersession chain

Acceptance gate:

- planner projection answers current belief for one object without raw spine scan
- operator projection explains why one claim is active

## Phase 6 Execution Coupling

Goal:

- make planning consume current world state deliberately

First coupling points:

- execution can map desired world change to claim delta
- planner can read current belief for target objects
- execution outcomes can publish object refs needed by world state

This is the first point where the knowledge graph stops being side analysis and becomes part of the cognitive loop.

Acceptance gate:

- one execution path reads planner-facing world state projection before acting
- one execution outcome changes current world state through replayable facts

## Minimum Test Set

These tests should exist by the end of the first landing:

- `domain_object_ref_round_trips`
- `legacy_spine_events_remain_readable_with_graph_fields`
- `world_state_event_builders_emit_explicit_object_refs`
- `execution_outcome_can_attach_claim_to_task_and_workspace_node`
- `replay_rebuilds_current_claim_projection`
- `late_fact_revises_claim_without_history_loss`
- `supersession_chain_remains_queryable`
- `planner_projection_reads_current_belief_without_spine_scan`

## File Map For First Landing

Shared spine contract work:

- `src/telemetry/events.rs`
- `src/telemetry/sinks/store.rs`
- `src/telemetry/routing/ingestor.rs`

New world state runtime area:

- `src/world_state.rs`
- `src/world_state/contracts.rs`
- `src/world_state/events.rs`
- `src/world_state/store.rs`
- `src/world_state/reducer.rs`
- `src/world_state/projection.rs`
- `src/world_state/query.rs`

Tests:

- `tests/integration/event_spine.rs`
- `tests/integration/world_state_graph.rs`

## Implementation Order

Use this order to reduce risk:

1. freeze `DomainObjectRef` and relation contracts
2. add optional graph fields to the spine with compatibility
3. create typed `world_state` records and emitters
4. land indexes before reducers
5. land reducers before planner reads
6. couple one execution path to planner-facing world state

This order ensures that current belief is never ahead of provenance and replay.

## Explicit Stop Conditions

Stop and redesign if any of these become true:

- the graph cannot be rebuilt from spine history
- object references cannot cross domains cleanly
- current claim reads require raw spine scans for common paths
- reducers need ad hoc mutable shortcuts outside replay
- planner coupling depends on filesystem identity as a universal anchor

## Read With

- [World State Domain](README.md)
- [Temporal Fact Graph](temporal_fact_graph.md)
- [Curation In World State](curation.md)
- [Spine Concern](../spine/README.md)
- [Multi-Domain Spine](../events/multi_domain_spine.md)
