# Execution Domain

Date: 2026-04-12
Status: active
Scope: world-model-aware action and the mapping from cognitive push into current execution areas

## Thesis

The system already has a strong push layer.
Within this repository, that layer is not one new monolith.
It is the composed behavior of `goals`, `control`, `task`, `capability`, `provider`, and domain-owned capability homes.

The cognitive architecture requirement is simple:
execution must read the current world model and must publish outcomes back into the spine.

## Boundary

`execution` owns:

- goal-directed planning against current belief
- dispatch through the existing execution domains
- publication of outcomes, failures, and learned facts back into the spine
- synthesis escalation when the current capability catalog cannot satisfy a goal

`goals` owns why change is needed.
`control` owns orchestration, observation waits, and repair.
`task` and `capability` own compiled execution units and atomic contracts.
`world_state` owns the world model that execution reads.

## Current Anchors

- [Goals](../../goals/README.md) already defines intent above `control`
- [Execution Control](control/README.md) defines `task_network`, planning structure, and repair
- [Synthesis Overview](control/synthesis/README.md) defines runtime capability growth

## Substrate

- [Execution Substrate](substrate.md)
  deliberate action over current belief through planning, control, task, and capability
- [Execution Control](control/README.md)
  active home for execution coordination and planning mechanics
- [Execution Planning](control/planning/README.md)
  now and next for HTN, planning, repair, and synthesis inside `execution`

## Required First Slice

- planner inputs sourced from the knowledge graph rather than only workspace state
- goal forms expressed as desired world-model change
- execution outcome records rich enough for curation to revise belief
- replanning rules for stale beliefs and changed observations

## Read With

- [Observe Merge Push](../observe_merge_push.md)
- [Execution Substrate](substrate.md)
- [Execution Control](control/README.md)
- [Execution Planning](control/planning/README.md)
- [World State Domain](../world_state/README.md)
- [Goals](../../goals/README.md)
- [Task Network](control/task_network.md)
- [Synthesis Overview](control/synthesis/README.md)
