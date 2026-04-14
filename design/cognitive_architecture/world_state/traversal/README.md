# Traversal

Date: 2026-04-13
Status: active
Scope: current anchor selection, lineage, provenance, and cross-domain graph walk inside `world_state`

## Thesis

`traversal` answers what is current and how to reach it.

This is already real in the repo today:

- workspace nodes provide structural anchors
- frames provide perspective-shaped knowledge artifacts
- heads provide the latest selected anchor for a node and frame type
- execution can move those anchors forward or fail to move them

The job of `world_state/traversal` is to lift that existing logic into explicit spine facts and graph-readable indexes without breaking the current system.

## What Traversal Owns

- current anchor selection
- anchor lineage
- provenance bundles
- cross-domain object walk through `DomainObjectRef`
- replayable graph materialization from spine facts

## What Traversal Does Not Own

- confidence in whether the current anchor is still correct
- contradiction handling
- calibration
- Bayesian revision

Those belong to `world_state/belief`.

## Current Operational Truth

The current operational graph already exists in these forms:

- `workspace_fs` node identity
- frame basis attachment
- head selection by node and frame type
- task and capability work that advances or preserves those heads

This means the first traversal branch is a lift and formalization task, not a greenfield invention.

## First Branch Scope

This branch should stop at traversal.

That means:

- event spine publication with explicit object refs and relations
- workspace and execution contribution to current anchor state
- replayable current anchor materialization
- query surfaces for current anchor, lineage, and provenance

Belief work is deferred.

## Read With

- [World State Domain](../README.md)
- [Belief](../belief/README.md)
- [Temporal Fact Graph](temporal_fact_graph.md)
- [Traversal Implementation Plan](implementation_plan.md)
- [Workspace FS Traversal Transition Requirements](workspace_fs_transition_requirements.md)
- [Spine Concern](../../spine/README.md)
