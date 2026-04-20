# Completed World State Graph

Date: 2026-04-20
Status: completed implementation archive
Scope: completed delivery trackers, migration notes, and graph implementation closeout records

## Purpose

This directory holds implementation history for the completed event spine and `world_state/graph` lift.

Declarative architecture remains under [World State Domain](../../../cognitive_architecture/world_state/README.md) and [Graph](../../../cognitive_architecture/world_state/graph/README.md).

Use this archive when you need delivery evidence, migration history, or branch-era rationale.
Use the active architecture docs when you need current domain boundaries and contracts.

## Completed Baseline

The completed first slice landed:

- canonical event spine graph fields
- runtime-wide event sequence and replay compatibility
- durable derived anchor facts
- traversal store indexes and query surface
- workspace scan and watch graph publication
- branch annotated federation
- workflow traversal consumption for task output resolution

## Documents

- [Graph Implementation Status](implementation_plan.md)
  completed baseline summary for graph contracts, runtime, reducers, queries, publishers, and verification
- [Spine Graph Completion Review](spine_graph_completion_plan.md)
  closeout review for event spine plus graph traversal
- [Spine Graph Feature Implementation Plan](spine_graph_feature_implementation_plan.md)
  checkpoint tracker and verification evidence
- [Workspace FS Graph Transition Status](workspace_fs_transition_requirements.md)
  completed workspace publication and compatibility record
- [Branch Federation Substrate Implementation Plan](branch_federation_substrate_implementation_plan.md)
  historical detailed branch substrate and federation plan
- [Branch Feature Implementation Plan](branch_feature_implementation_plan.md)
  completed branch CLI, dormant workflow, migration, and federated read tracker
- [Branch Lift Plan](branch_lift_plan.md)
  historical roots to branches lift plan
- [Roots Retrofit Plan](roots_retrofit_plan.md)
  completed root retrofit tracker
- [Root Migration First Slice](root_migration_first_slice.md)
  historical first slice for root registration and migration bookkeeping
- [Root Federation Runtime](root_federation_runtime.md)
  historical root federation runtime design
- [Root Migration Architecture](root_migration_architecture.md)
  historical root migration architecture note

## Active Design

- [World State Domain](../../../cognitive_architecture/world_state/README.md)
- [Graph](../../../cognitive_architecture/world_state/graph/README.md)
- [Temporal Fact Graph](../../../cognitive_architecture/world_state/graph/temporal_fact_graph.md)
- [Branch Federation Substrate](../../../cognitive_architecture/world_state/graph/branch_federation_substrate.md)
- [Belief](../../../cognitive_architecture/world_state/belief/README.md)
