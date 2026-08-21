# World Model Public Interface

The public interface exposes domain contracts rather than storage internals.

## Graph And Traversal

Owners publish addressable objects and typed relation occurrences with stable identity, provenance, currentness, temporal scope, branch scope, and owner revision.

Traversal accepts a root set, relation policy, scope, cut identity, and resource bounds. It returns reached objects and relation occurrences without erasing occurrence identity or owner qualification.

## Belief

Belief accepts evidence candidates, applies Agent perspective and comparator policy, and publishes immutable revisions. Queries return a revision-bound view with supporting and opposing evidence.

## Planner

Planner materializes an immutable `PlannerCut` from exact graph, belief, causal, regime, Capability, and directive revisions. It does not choose actions or authorize work.

## Curation

Curation accepts an Agent-authorized `EpistemicOperation`. It validates operation shape, authority, idempotency, and the graph state it owns. It emits a durable `EpistemicOperationResult` and owner-shaped graph publications through Events.

## Strategy

Strategy accepts a ground Goal, directive context, and one immutable planner cut. It returns a `StrategyPlan` containing desired conditions, complete Tasks, bounded Epistemic Operations, causal dependencies, frozen context identity, and lineage.

Strategy verification proves the Plan's internal semantic closure. It does not authorize or execute the Plan.

## Agent

Agent accepts admitted belief changes, Goal lifecycle facts, Curation results, and execution outcomes. It may construct or reconcile a Plan, authorize exact Plan products, route eligible epistemic work to Curation, route eligible complete Tasks to Execution, suspend work, or declare a Goal satisfied through the owning satisfaction contract.

Agent decisions are durable and idempotent. Every routed product retains Plan, Goal, Agent, context, and producer lineage.
