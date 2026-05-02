# Branch Federation Substrate

Date: 2026-04-20
Status: completed implementation archive
Scope: declarative branch federation model for graph traversal

## Thesis

`branch` is the durable scope for graph presence and traversal.

`workspace_fs` is the first branch kind, but the substrate must also support future non filesystem branch kinds without changing the federation contract.

Federation is not one merged physical graph store.
It is a scoped read model over branch local source truth, branch local derived projections, and one shared graph contract.

## Core Contract

Every branch has:

- stable branch identity
- branch kind
- locator metadata
- branch local source truth
- branch local graph projections
- migration and health metadata
- graph catch up state

Branch identity is authoritative.
Locator metadata is mutable.

## Branch Presence

Federation must preserve the difference between semantic object identity and branch presence.

One `DomainObjectRef` can be present in many branches.
Each presence row must retain:

- branch id
- canonical locator
- object ref
- first seen sequence
- last seen sequence
- current branch presence

This prevents a federated read from flattening local provenance into one ambiguous object.

## Query Scope

Federated reads must be explicitly scoped.

Supported scopes:

- active branch
- selected branch ids
- all readable branches

Every federated traversal must keep:

- depth cap
- direction
- relation filters
- current only mode
- branch annotated facts and relations
- skipped branch metadata

## Authority Rules

- spine facts remain the temporal source for graph materialization
- branch local source truth remains authoritative for that branch
- derived traversal stores are rebuildable
- federation reads branch local projections
- federation does not rewrite branch local source truth
- unreadable branches are surfaced, not silently erased

## Relationship To Graph

`world_model/graph` owns local traversal materialization.

`branches` owns scope selection and federated read composition.

The graph query contract must remain useful for a single branch before federation composes it across many branches.

## Relationship To Belief

Branch federation answers where graph facts and objects are present.

It does not decide whether a current anchor is credible.
Belief and curation decide confidence, contradiction, settlement, and calibration above the federated graph surface.

## Archive Note

This document records the completed branch federation substrate rationale.

The active graph contract now lives in [World Model Graph](../../../cognitive_architecture/world_model/graph/README.md). Branch-scoped reads remain part of that active contract, but the detailed substrate plan belongs with the completed graph implementation archive.

## Read With

- [Completed World State Graph](README.md)
- [Branch Federation Substrate Implementation Plan](branch_federation_substrate_implementation_plan.md)
- [Branch Feature Implementation Plan](branch_feature_implementation_plan.md)
- [World Model Graph](../../../cognitive_architecture/world_model/graph/README.md)
- [Belief](../../../cognitive_architecture/world_model/belief/README.md)
