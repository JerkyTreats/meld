# Execution Planning

Date: 2026-07-23
Status: active
Scope: planning structure inside execution

## Thesis

Planning lives inside `execution`.
It reads admitted Goals, their nonempty authorized Strategy inventories, the `WorldState` projected by the world model, and live task-network state. It then submits commands that realize an authorized theory of action as committed operational work.

The foundational pattern is graphs-lower-graphs: capabilities compose into tasks, tasks compose into the task network. The planning loop continuously checks authorized concrete Compositions for current applicability, tunes realization against live state, and issues task-network commands when the operational plan should change.

The shared typed language [`meld-lang`](../../meld-lang/README.md) provides the substrate for all planning operations. Goals are `Proposition` targets evaluated against `WorldState`. Strategy-authorized concrete Compositions are the episode-specific input. A candidate may preserve a complete inventory of exact Method-instance derivations, but Strategy has already instantiated them. Execution wraps the concrete Composition with planning context, validation reports, projected effects, and operator resolution reports. Operators resolve to capabilities through the catalog. The planning loop is mechanical and never invents semantic intent.

Strategy owns causal candidate construction. Execution Planning owns applicability, exact allowed bindings, live operational selection, capability resolution, reuse, lowering, task-network diffing, and commitment. Rejection of a stale or unsupported candidate returns typed facts for renewed Strategy. It does not authorize semantic repair inside Execution.

Execution Planning does not decide whether a Goal has any semantic theory of action. World-model Strategy proves that before admission. The current `NoApplicableMethod` result remains a mechanical boundary failure when an admitted inventory cannot be realized.

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
- [World Model Strategy](../../world_model/strategy/README.md)
