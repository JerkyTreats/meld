# Graph

Date: 2026-04-30
Status: active
Scope: current anchor selection, lineage, provenance, traversal, and branch-scoped graph reads inside the world model

## Thesis

`world_model/graph` answers what is current, how it became current, where it is present, and how to reach related objects.

This is the current Meld substrate for the rest of the world model. It is not the belief layer, a causal graph, a regime detector, or a planner. It is the replayable bitemporal graph surface that those layers consume.

The graph contract is:

- canonical events carry `DomainObjectRef` objects and `EventRelation` edges
- graph materialization derives current anchors and traversal views from event history
- anchors are selected for a subject and perspective
- lineage records what an anchor replaced
- provenance records which facts made an anchor current
- traversal exposes object history, relation adjacency, and bounded walks
- branch-scoped reads preserve branch identity and presence
- derived anchor facts are written back to the event spine idempotently

The design work now is to keep this surface narrow, replayable, index backed, and explicit enough for belief, causation, regime, and planner-facing projection layers to use without reaching into source-domain internals.

## Current Outcome

The graph baseline is largely achieved in code today.

It includes:

- canonical event publication with explicit object refs and relations
- workspace, context, execution, task, and artifact contribution to current anchor state where those domains publish graph-readable facts
- replayable current anchor materialization
- query surfaces for current anchor, lineage, provenance, neighbors, object history, and bounded walks
- durable derived anchor events written back to the event ledger by `GraphRuntime`
- branch-annotated federation over traversal stores
- workflow task paths that can resolve final frame artifacts through traversal

The remaining active work is mostly crate boundary cleanup: continuing the breakout from a singular monolith into explicit domain crates while preserving the same graph contract.

## What Graph Owns

- current anchor selection
- anchor lineage and supersession chains
- provenance bundles for current anchors and traversal facts
- cross-domain object identity through `DomainObjectRef`
- typed relation adjacency through `EventRelation`
- bounded graph walks over materialized traversal indexes
- object history and source fact hydration handles
- branch presence and branch-scoped federated reads
- replayable graph materialization from event facts

## What Graph Does Not Own

- confidence in whether the current anchor is still correct
- contradiction handling
- calibration
- Bayesian revision
- causal identification
- regime inference
- planner policy or task dispatch

Those responsibilities belong to upper world model layers and `execution`.

## Surface For Other Layers

### Belief

Belief consumes graph output as evidence input.

The useful graph surfaces are:

- current anchor for a subject and perspective
- supporting provenance facts for that anchor
- lineage and supersession chain for replacement reasoning
- object history for freshness and observation coverage
- bounded walks for related evidence discovery
- source fact identifiers for audit and evidence hydration

Graph still stops before settlement. An anchor says what is currently selected; belief decides whether that selection is trusted, contradicted, stale, provisional, or invalid.

### Causation

Causation consumes graph output as temporal and relational evidence.

The useful graph surfaces are:

- state before and after an intervention-shaped fact
- relation paths between interventions, measured states, and outcomes
- provenance for which measurement made a state visible
- selection paths that may explain why one anchor or report was chosen
- branch scope for separating local histories

Graph must not imply causality from temporal order, reachability, or anchor replacement. It only provides the auditable state and relation structure that causal inference can reason over.

### Regime

Regime consumes graph and belief signals to detect structural change.

The useful graph surfaces are:

- anchor churn by subject or relation family
- relation stability and disappearance
- observation cadence by object or branch
- branch-local divergence in current anchors
- repeated provenance patterns behind failures or reversions

Graph does not decide when a changepoint occurred. It makes the temporal and relational signals available for regime inference.

### Planner-Facing Projection

The planner-facing world model should usually consume belief, causal, and regime views rather than raw graph reducer state.

Graph remains useful to planner-facing reads for:

- hydration handles after a planner-relevant view has been selected
- provenance summaries for explanation
- current object walks needed to construct concrete task inputs
- branch-scoped reads when a decision is local to a branch

Planning should not scan the event spine or graph indexes to settle belief during action selection.

## Query Families

The graph layer should preserve these query families as its public design surface:

- current anchor by subject and perspective
- current workspace source, frame head, task artifact, or other graph-readable selector
- anchor provenance by anchor id
- anchor lineage and supersession chain
- object history by `DomainObjectRef`
- facts mentioning an object after a sequence
- relation neighbors by direction and relation filter
- bounded graph walk with depth, direction, relation filters, current-only selection, and optional fact inclusion
- branch presence for an object
- branch-annotated federated traversal

These are the graph affordances higher layers can rely on. New graph indexes should exist to serve these families, not to create an unbounded graph database surface.

## Remaining Limits

- world-model belief is outside the graph baseline
- confidence, contradiction handling, calibration, and curation are not graph responsibilities
- causation and regime layers must treat graph output as evidence, not as settled mechanism or structural-break proof
- traversal only reduces source domains that publish explicit graph objects and relations
- graph indexes are sled-backed materializations, not a general graph database
- branch federation preserves per-branch presence and provenance, but does not merge branch facts into one global authority

## Read With

- [World Model Domain](../README.md)
- [Graph ECS](ECS.md)
- [World Model Crate](../CRATE.md)
- [Belief](../belief/README.md)
- [Causal Layer](../causation/README.md)
- [Regime Layer](../regime/README.md)
- [Planner Projection](../planner/README.md)
- [Events Design](../../events/README.md)
- [Completed Temporal Fact Graph](../../../completed/world_state/graph/temporal_fact_graph.md)
- [Completed Branch Federation Substrate](../../../completed/world_state/graph/branch_federation_substrate.md)
