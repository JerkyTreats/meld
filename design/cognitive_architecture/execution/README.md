# Execution

Execution is Meld's domain for realizing authorized executable work. It owns the Goal Set, execution coherence, lowering, dispatch, progress, and the unified Task Network.

Execution answers How and When. It does not decide What the Agent should want or Why a Task advances a directive.

## Intake

Execution accepts a producer-neutral Goal-attributed Task admission. The Task is complete before admission and carries exact Capability contracts, bindings, internal dependencies, authority, and lineage.

Any authorized producer may submit a valid Task. Execution does not require Strategy, belief, candidate graph, planner cut, Capability-selection proof, or Epistemic Operation context.

Execution validates transport shape and protects its own authority, live Capability availability, idempotency, concurrency, resource, persistence, and Task Network invariants. It does not reconstruct the producer's semantic reasoning.

## Execution Planning

Execution Planning considers all admitted executable products together. It may reuse one action for several Goals, rank priorities, sequence dependent work, parallelize independent work, reserve resources, and lower accepted Tasks into one operational Task Network.

If two Agents independently request the same Merkle scan over the same workspace revision, Execution may realize both obligations with one compatible action and attribute the result to both admissions.

Execution Planning never invents a missing semantic step. An incomplete Task is rejected at intake rather than completed through world-model reasoning.

## Outcomes

Task and action outcomes are appended through Events. Execution records what ran, when it ran, what artifacts and observations it produced, and which admitted obligations it discharged. It does not declare the originating Goal epistemically satisfied.

## Domain Documents

- [Goal Set](goals/README.md)
- [Execution Planning](planning/README.md)
- [Task Network](task_network.md)
- [Capabilities](capabilities.md)
- [Task Initialization](task_initialization.md)
- [Failure Signals](failure_signal.md)
