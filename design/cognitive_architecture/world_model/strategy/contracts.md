# Strategy Contracts

## Input

A Strategy construction request carries a ground Goal, Agent and directive identity, an immutable planner cut, the complete active Capability catalog, and optional predecessor Plan lineage.

The Goal target remains immutable during one construction. The planner cut binds every semantic input to exact revisions and scope.

## Output

A `StrategyPlan` carries Plan identity, Goal identity, Agent identity, desired conditions, complete Tasks, bounded Epistemic Operations, dependency edges, frozen context identity, explanation, and predecessor lineage.

Each Task proves Capability contract identity, required input bindings, artifact flow, internal dependency closure, and expected executable outcome.

Each Epistemic Operation proves bounded target scope, requested graph authorship, authority, idempotency key, expected epistemic result, and publication policy.

## Verification

Strategy verification checks only Strategy-owned meaning. It verifies grounding, closure, causal coherence, dependency acyclicity where required, frozen-cut consistency, and explicit handling of uncertainty.

Verification does not grant Agent authority, reserve execution resources, append Curation facts, or predict that external effects have occurred.

## Consumer Contracts

Agent consumes the full Plan and owns authorization plus progression.

Curation consumes only an authorized Epistemic Operation with its Goal and Plan lineage.

Execution consumes only an authorized complete Task with Goal and Plan lineage.

Neither consumer receives Strategy's private search state or revalidates Strategy's full semantic proof.
