# Execution Planning

Date: 2026-05-09
Status: active
Scope: planning structure inside execution

## Thesis

Planning lives inside `execution`.
It reads the goal set (curated by world model agents) and the world model view (subscribed through the agent's perspective), then maintains a task network graph that closes the gap between current belief and desired state.

The foundational pattern is graphs-lower-graphs: capabilities compose into tasks, tasks compose into the task network. The planning loop continuously decomposes goals via HTN methods and issues graph mutations to the task network when the plan should change.

## Documents

- [Planning Pipeline](planning_pipeline.md)
  graphs-lower-graphs execution model: planning loop + task network, connected by graph mutations
- [Guard Expression Semantics](guard_expression_semantics.md)
  evaluation rules for guard expressions on conditional dependency edges
- [Observation Wait Semantics](observation_wait_semantics.md)
  runtime semantics for observation tasks as data-flow dependencies

## HTN Model

- [HTN Model](htn/README.md)
  abstract task identity, method identity, decomposition boundaries, and lineage
- [HTN Lineage Model](htn/lineage_model.md)
  durable hierarchy records for scoping plan changes, explaining execution, and guiding method reselection

## Read With

- [Execution Domain](../README.md)
- [Goals](../goals/README.md)
- [Task Network](../task_network.md)
- [Synthesis Overview](../synthesis/README.md)
- [World Model Planner](../../world_model/planner/README.md)
- [World Model Agent](../../world_model/agent/README.md)
