# Execution Planning

Execution Planning lowers the current Goal Set into one coherent Task Network. It owns operational decisions about reuse, priority, sequence, parallelism, resources, and readiness.

It does not perform Strategy decomposition, inspect belief, traverse the knowledge graph, create Epistemic Operations, or determine why a Goal matters.

## Inputs

Planning reads admitted complete Tasks, Capability runtime availability, Task Network history, resource state, effect compatibility, and execution policy.

## Coherence

Planning may merge compatible action instances, preserve separate attribution, order conflicting effects, parallelize independent actions, and reuse completed artifacts whose identity and validity still satisfy an admission.

Coherence never changes the semantic obligation of an admitted Task. If operational constraints make realization impossible, Execution reports that fact to the producer.

## Lowering

Each admitted Task compiles into operational nodes and edges. Compilation preserves Task identity, Capability contract identity, artifact bindings, dependencies, authority, Goal attribution, and producer lineage.

The unified Task Network may contain shared operational nodes with several admission attributions. Shared realization is legal only when effects and results are semantically compatible under Execution-owned rules.

## Reconciliation Boundary

Execution may retry, reschedule, or re-lower work within its operational authority. A semantic change to What should be done returns as an outcome and is reconciled by the owning Agent and Strategy domains.

- [Planning Pipeline](planning_pipeline.md)
- [Observation Wait Semantics](observation_wait_semantics.md)
- [Guard Expression Semantics](guard_expression_semantics.md)
