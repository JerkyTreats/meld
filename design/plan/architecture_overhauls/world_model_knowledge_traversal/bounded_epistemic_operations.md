# Bounded Epistemic Operations In Strategy Plans

> Superseded for current reconciliation direction. [Canonical Flywheel Alignment And Runtime Soft Freeze](flywheel_remediation.md) alone owns the issue register, priorities, next work, and branch acceptance. This document retains historical design or evidence; its prior status and authorization statements do not govern current delivery.

Date: 2026-08-20

Status: discovery refinement, implementation not authorized

## Finding

The assertion is supported with one qualification:

> An effective goal-closing Strategy plan may require both epistemic operations and executable Tasks.

An executable Task changes the observable world. A bounded epistemic operation changes the admitted knowledge through which an Agent understands that world.

Neither is a substitute for the other. Strategy determines which causal events are required and how they depend on one another. Curation performs epistemic operations. Execution performs Tasks.

The current one-Task Strategy contract cannot represent this mixed causal structure. Its prospective evidence route describes later reconciliation, but it is not a first-class epistemic operation that can precede, follow, or gate a Task.

## Domain Separation

```text
Traversal
-> reads materialized knowledge

Curation
-> executes bounded epistemic operations
-> authors Curation-owned entities and edges through Events

Strategy
-> constructs a causal plan
-> constructs each Epistemic Operation and Task as an isolated closed product
-> chains those products by semantic dependency

Execution
-> receives Tasks only
-> lowers them into the Task Network
```

Traversal is not Curation. Curation is not Strategy. Strategy is not Execution.

Strategy authors the plan and the ground operation requests. It does not execute either kind of step. Each executing domain retains the mechanics and semantics of its own product.

## Two Uses Of Curation

Curation appears on both sides of Goal formation.

Standing Curation applies an Agent specification to admitted world knowledge. It can author expected entities, requirements, materiality relationships, and bounded realization assessments. This is how an Agent first obtains an explicit mismatch from which to form a Goal.

Strategy-planned Curation executes a bounded epistemic operation named in a Strategy plan. It can obtain or reconcile the epistemic result needed before a Task is eligible, or assess admitted observations after a Task completes.

```text
Agent specification
-> standing Curation
-> explicit expected-versus-observed mismatch
-> Goal
-> Strategy plan
-> bounded Curation and Task steps
```

These are two invocation contexts for the same domain. They do not justify two curation authorities.

## Bounded Epistemic Operation

A bounded epistemic operation is a closed request to derive, connect, compare, or assess knowledge under one exact Agent perspective.

Its semantic input includes:

- exact Agent and perspective identity
- exact operation or rule revision
- exact root objects
- a frozen knowledge cut or source cursor
- traversal direction, relation filters, and bounds
- the Curation-owned vocabulary it may author
- explicit completion, abstention, and incompleteness meaning

Its output may include:

- new expected entities
- new epistemic edges
- a realization or coverage assessment
- an unchanged result
- an abstention
- a bounded incomplete result with exact limits

It does not contain a Capability graph, provider choice, execution schedule, Task Network command, or external-world effect.

Curation publishes its result through Events. Traversal materializes the resulting nodes and edges. Curation does not mutate Traversal storage directly.

## Strategy Plan

The proposed Strategy product implied by this model sits above Task.

```text
StrategyPlan
-> EpistemicStep
   -> BoundedEpistemicOperation
-> ExecutableStep
   -> Task
-> causal dependencies between steps
```

Each step type has isolated closure.

Task construction closes Capability identities, bindings, artifacts, and executable dependencies.

Epistemic-operation construction closes knowledge roots, traversal bounds, rule identity, perspective, admissible output vocabulary, and completion meaning.

Strategy Planning composes these closed products according to causal need. It does not flatten an epistemic operation into a Task or wrap a Task in an epistemic abstraction.

The plan graph expresses semantic dependency rather than an Execution schedule. Execution still owns priority, parallelism, sequencing, reuse, and Task Network lowering among admitted Tasks.

## README Example

The standing path is:

```text
folder F
+ Agent rule that every folder requires a correct README
-> Curation authors expected README F
-> Curation compares expected README F with workspace snapshot S
```

If an observed README realizes the expected entity and admitted claim evidence supports correctness, Curation can establish the epistemic relationship needed for the Agent to close the loop. No executable Task is required.

If no observed README realizes the expected entity, Strategy can construct a mixed plan:

```text
Epistemic Operation
-> identify claims required in expected README F

Task
-> create or revise the physical README

Epistemic Operation
-> map observed README claims to required source claims
-> assess realization and coverage under the new workspace snapshot
```

The final epistemic operation does not claim that Task completion proves correctness. It consumes the admitted post-Task observation and authors the Curation-owned assessment. Agent satisfaction remains a separate judgment over that reconciled state.

## Epistemic-Only And Action-Only Plans

An epistemic-only plan is valid when the necessary observations already exist and the missing causal work is derivation, connection, reconciliation, or assessment. The README already exists and is correct, but the knowledge graph has not yet connected the evidence, is the anchor example.

An action-only candidate is mechanically possible. Strategy can propose a Task that changes the world without embedding an epistemic step in that Task.

An action-only plan is not independently goal-closing when Goal satisfaction is observational. It cannot establish that its action produced the desired state. That epistemic closure may be provided by standing Curation outside the candidate, or it must appear as a bounded epistemic step in the Strategy plan.

Therefore the strong assertion is not that every Strategy plan must contain both step types. It is:

> Every effective plan must account for the epistemic route from expected state through admitted observation to satisfaction. When that route requires causal work, epistemic operations must be first-class plan steps.

## Current Architecture Conflict

Current design and code center successful Strategy search on one `StrategyCandidate` containing one executable `Composition` and one `ProspectiveEvidenceRoute`.

Implemented primitives support:

- pure bounded Strategy search
- one immutable world-state input
- one ground executable Composition
- prospective evidence metadata
- independent Agent verification and authorization

They do not support:

- a heterogeneous Strategy plan
- a bounded epistemic operation contract
- epistemic step construction
- causal dependencies between epistemic and executable products
- plan progression across Curation and Execution outcomes

Adopting this model would change the canonical Strategy product. It would not change Execution planning ownership. Execution can remain ignorant of epistemic operations and receive only each authorized Task that becomes eligible.

## Producer Boundaries

The ownership policy remains coherent:

- Curation defines and enforces bounded epistemic-operation semantics
- Capability owners define atomic executable contracts
- Strategy constructs ground operation and Task instances and proves the causal plan
- Agent authorizes the exact plan or its eligible products
- Curation executes epistemic operations and authors epistemic results
- Execution lowers and processes Tasks
- observational owners publish world facts
- Agent judges Goal satisfaction

Strategy may ground a Curation operation contract. It must not invent Curation relation meaning, just as it may ground a Capability contract without inventing Capability behavior.

## Open Design Threads

- whether the canonical product is named `StrategyPlan`, `StrategyCandidate`, or another Strategy-owned term
- whether Agent authorizes the whole causal plan, each enabled product, or both at different authority levels
- whether one Goal may publish several Tasks over the life of one Strategy plan
- whether plan progression belongs to Agent runtime or a distinct world-model plan runtime
- whether Strategy plans are fully expanded or may pause after an epistemic step and be reconstructed from the new snapshot
- the public contract through which Curation advertises constructible epistemic operations to Strategy
- how causal dependencies distinguish required knowledge, enabling knowledge, validation, and satisfaction evidence
- how standing Curation and Strategy-planned Curation share replay and deduplication without conflating their initiating authority

These are discovery questions. The model does not authorize changes to Execution, Task Network, or `meld-lang`.

## Evidence

- [current Strategy thesis](../../../cognitive_architecture/world_model/strategy/README.md)
- [current Strategy search model](../../../cognitive_architecture/world_model/strategy/search.md)
- [current Strategy contracts](../../../../crates/meld-world-model/src/strategy/contracts.rs)
- [docs freshness example](../../../cognitive_architecture/world_model/strategy/docs_freshness.md)
- [Strategy Plan redesign](strategy_plan_redesign.md)
- [Traversal and Curation assessment](curation_assessment.md)
