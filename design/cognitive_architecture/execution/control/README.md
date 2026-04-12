# Execution Control

Date: 2026-04-09
Status: active
Scope: execution coordination, planning mechanics, task-network state, and repair inside `execution`

## Thesis

`control` is the coordination subsystem inside `execution`.
It is not a peer cognitive domain.

This area is the active home for detailed mechanics such as:

- task-network state
- event reduction
- control-program semantics
- continuation
- repair
- HTN lineage
- synthesis escalation

Read this directory as the detailed execution-control layer under `execution`.

## Read Order

1. [Execution Domain](../README.md)
2. [Execution Substrate](../substrate.md)
3. [Planning](planning/README.md)
4. [Task Network](task_network.md)
5. [Synthesis Overview](synthesis/README.md)
6. [External Process Capability](synthesis/external_process_capability.md)
7. [Synthesis Task](synthesis/synthesis_task.md)
8. [Runtime Catalog](synthesis/runtime_catalog.md)
9. [HTN Model](planning/htn/README.md)
10. [HTN Lineage Model](planning/htn/lineage_model.md)
11. [Control Program Model](program/README.md)
12. [Control Graph Model](program/control_graph.md)
13. [Await Observation Semantics](program/await_observation_semantics.md)
14. [Guard Binding Semantics](program/guard_binding_semantics.md)
15. [Runtime Model](runtime/README.md)
16. [Continuation Model](runtime/continuation_model.md)
17. [Repair Model](repair/README.md)
18. [Repair Entry Model](repair/repair_entry.md)

## Read With

1. [Goals](../../../goals/README.md)
2. [World State Domain](../../world_state/README.md)
3. [Spine Concern](../../spine/README.md)
4. [Capability And Task Design](../../../capabilities/README.md)
5. [Events Design](../../events/README.md)
6. [Multi-Domain Spine](../../events/multi_domain_spine.md)

## Now

- task and capability are the active deliberate execution substrate in code
- task-network and event reduction are the clearest execution-coordination direction
- the event spine and telemetry refactor work are active enabling design

## Next

- make planning explicitly world-state-aware
- land durable task-network runtime state
- connect observation waits to a generic information-gathering policy
- keep HTN as one planning structure without forcing all planning into one formalism
