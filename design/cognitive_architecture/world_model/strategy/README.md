# Strategy

Strategy is the pure world-model domain for Plan construction and reconstruction. It answers what conditions must become true and why those conditions reconcile a ground Goal with admitted world state.

Strategy does not authorize work, mutate Goal lifecycle, perform epistemic operations, invoke Capabilities, schedule Tasks, or know the Task Network.

## Strategy Plan

A `StrategyPlan` is an immutable heterogeneous causal graph. It contains:

- desired conditions and their satisfaction semantics
- complete executable Tasks
- bounded Epistemic Operations
- causal and information dependencies
- exact frozen context identity
- producer and derivation lineage
- eligibility conditions for each product

A Task is a closed graph of bound Capability invocations. It is one executable discharge product, not the Plan.

An Epistemic Operation is a bounded request to author shared knowledge. It is not a Capability and is never sent to Execution.

A desired condition may be discharged by epistemic authorship, external execution, later observation, or a causal chain combining them.

## Construction

Strategy begins from a ground Goal and one immutable planner cut. It traverses relation-rich context to identify relevant entities, evidence, claims, mechanisms, constraints, and available Capabilities. It decomposes the mismatch until every leaf is either already satisfied, a complete Task, a bounded Epistemic Operation, or an explicitly unresolved condition.

A Plan is eligible for Agent judgment only when each selected product has exact inputs, bindings, authority requirements, dependencies, and expected outcomes. Unresolved conditions may remain only when the Plan explicitly represents how later knowledge can resolve them.

## Reconstruction

When admitted knowledge or outcomes invalidate a premise, Agent asks Strategy to construct a successor Plan from a new immutable cut and preserved causal history. Strategy may retain, replace, add, or remove uncommitted products. It never rewrites completed facts.

## Explanation

Every Plan can explain why each condition matters, why each product was selected, which evidence and mechanisms support it, and which later facts would cause reassessment.

## Boundaries

Strategy may depend on the world-model public contracts and permissive values from `meld-lang`. Its Plan aggregate remains world-model-owned. No language-layer guard attempts to enforce Strategy grammar.

- [Contracts](contracts.md)
- [Plan Construction](search.md)
- [Docs Freshness](docs_freshness.md)
- [Dependency Security](cve_freshness.md)
