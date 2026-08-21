# World State Values

`WorldState` is an immutable set of ground propositions for pure language evaluation. It is a value representation, not the world model itself.

The world-model Planner may project a revision-bound subset into `WorldState` when a language operation requires it. That projection preserves its external cut identity in the owning domain aggregate.

Pure evaluation returns satisfied, unsatisfied, or indeterminate. Indeterminate means the value set does not contain enough information. It does not authorize observation, Curation, Task construction, or execution.

Execution need not receive a world-model projection with Task admission. It evaluates only executable guards whose required values are supplied through Execution-owned artifacts and runtime state.
