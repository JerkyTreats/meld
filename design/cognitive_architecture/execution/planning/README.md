# Execution Planning

Date: 2026-06-02
Status: active
Scope: planning structure inside execution

## Thesis

Planning lives inside `execution`.
It reads the goal set curated by world model agents and the `WorldState` projected by the world model through the shared language, then submits commands that maintain a task network graph that closes the gap between current belief and desired state.

The foundational pattern is graphs-lower-graphs: capabilities compose into tasks, tasks compose into the task network. The planning loop continuously decomposes goals via HTN methods and issues task network commands when the plan should change.

The shared typed language [`meld-lang`](../../meld-lang/README.md) provides the substrate for all planning operations. Goals are `Proposition` targets evaluated against `WorldState`. Methods match goals through pattern unification and produce raw `Composition` graphs. Execution wraps a concrete `Composition` as an execution composition when planning context, validation reports, projected effects, and operator resolution reports are attached. Operators resolve to capabilities through the catalog. The planning loop is mechanical — it never interprets semantic intent.

## Documents

- [Planning Pipeline](planning_pipeline.md)
  graphs-lower-graphs execution model: planning loop and task network, connected by commands
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
- [Execution Gaps](../GAPS.md)
- [Goals](../goals/README.md)
- [HTN Model](htn/README.md)
- [HTN Lineage Model](htn/lineage_model.md)
- [Task Network](../task_network.md)
- [Synthesis Overview](../synthesis/README.md)
- [Guard Expression Semantics](guard_expression_semantics.md)
- [Observation Wait Semantics](observation_wait_semantics.md)
- [Lang Domain](../../meld-lang/README.md)
- [Lang Goals and Methods](../../meld-lang/goals_and_methods.md)
- [Lang Operators and Resolution](../../meld-lang/operators.md)
- [Lang Compositions](../../meld-lang/compositions.md)
- [Lang World State and Evaluation](../../meld-lang/world_state.md)
- [World Model Planner](../../world_model/planner/README.md)
- [World Model Agent](../../world_model/agent/README.md)
